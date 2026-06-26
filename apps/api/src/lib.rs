use aide::{
    axum::{
        ApiRouter, IntoApiResponse,
        routing::{get, post_with},
    },
    openapi::{Info, OpenApi},
};
use axum::{
    Extension, Json, Router,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use engine::{config::SimulationConfig, simulation::Simulation};
use schemars::JsonSchema;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, JsonSchema)]
pub struct ApiErrorBody {
    pub error: String,
}

#[derive(Clone, Default)]
struct AppState {
    run_status: Arc<Mutex<SimulationRunStatus>>,
}

#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SimulationRunStatus {
    #[default]
    Idle,
    Running {
        generation: u32,
        total_generations: u32,
    },
    Complete {
        generation: u32,
        total_generations: u32,
    },
    Failed {
        error: String,
    },
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiErrorBody>)>;

pub fn router() -> Router {
    let mut api = openapi_document();
    let state = AppState::default();

    documented_router()
        .finish_api(&mut api)
        .route("/openapi.json", get(serve_openapi).into())
        .layer(Extension(api))
        .with_state(state)
}

pub fn generate_openapi() -> OpenApi {
    let mut api = openapi_document();
    let _router = documented_router().finish_api(&mut api);
    api
}

fn openapi_document() -> OpenApi {
    OpenApi {
        info: Info {
            title: "Schism API".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: Some("Thin HTTP API for the Schism simulation engine.".to_owned()),
            ..Info::default()
        },
        ..OpenApi::default()
    }
}

fn documented_router() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/simulations/run",
            post_with(run_simulation, |operation| {
                operation
                    .id("runSimulation")
                    .description(
                        "Run a Schism simulation using a partial SimulationConfig JSON body.",
                    )
                    .response::<200, Json<engine::simulation::SimulationReadout>>()
                    .response::<400, Json<ApiErrorBody>>()
                    .response::<409, Json<ApiErrorBody>>()
                    .response::<500, Json<ApiErrorBody>>()
            }),
        )
        .api_route("/simulations/status", get(get_simulation_status))
}

async fn serve_openapi(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api)
}

async fn run_simulation(
    State(state): State<AppState>,
    payload: Result<Json<SimulationConfig>, JsonRejection>,
) -> ApiResult<engine::simulation::SimulationReadout> {
    let Json(config) = payload.map_err(|rejection| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorBody {
                error: rejection.body_text(),
            }),
        )
    })?;

    config.validate().map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorBody {
                error: err.to_string(),
            }),
        )
    })?;

    let total_generations = config.world.num_generations;
    {
        let mut run_status = state.run_status.lock().expect("run status lock poisoned");
        if matches!(*run_status, SimulationRunStatus::Running { .. }) {
            return Err((
                StatusCode::CONFLICT,
                Json(ApiErrorBody {
                    error: "a simulation is already running".to_owned(),
                }),
            ));
        }

        *run_status = SimulationRunStatus::Running {
            generation: 0,
            total_generations,
        };
    }

    let status_for_task = state.run_status.clone();
    let readout = tokio::task::spawn_blocking(move || {
        let mut simulation = Simulation::new(config)?;
        simulation.run_to_readout_with_progress(|generation, total_generations| {
            *status_for_task.lock().expect("run status lock poisoned") =
                SimulationRunStatus::Running {
                    generation,
                    total_generations,
                };
        })
    })
    .await
    .map_err(|err| {
        *state.run_status.lock().expect("run status lock poisoned") = SimulationRunStatus::Failed {
            error: format!("simulation task failed: {err}"),
        };

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorBody {
                error: format!("simulation task failed: {err}"),
            }),
        )
    })?
    .map_err(|err: anyhow::Error| {
        *state.run_status.lock().expect("run status lock poisoned") = SimulationRunStatus::Failed {
            error: err.to_string(),
        };

        (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorBody {
                error: err.to_string(),
            }),
        )
    })?;

    *state.run_status.lock().expect("run status lock poisoned") = SimulationRunStatus::Complete {
        generation: total_generations,
        total_generations,
    };

    Ok(Json(readout))
}

async fn get_simulation_status(State(state): State<AppState>) -> Json<SimulationRunStatus> {
    Json(
        state
            .run_status
            .lock()
            .expect("run status lock poisoned")
            .clone(),
    )
}
