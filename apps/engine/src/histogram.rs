use rand_distr::num_traits::ToPrimitive;

#[derive(Debug)]
pub enum HistogramError {
    CellNotFound(HistogramCell),
    WouldGoNegative { cell: HistogramCell, current: u64, delta: i64 },
}

impl std::fmt::Display for HistogramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistogramError::CellNotFound(cell) => write!(
                f,
                "cell not found: heterodoxy_bin={}, age_bin={}",
                cell.heterodoxy_bin.value(),
                cell.age_bin.value()
            ),
            HistogramError::WouldGoNegative { cell, current, delta } => write!(
                f,
                "adjustment would go negative: cell=({}, {}), current={}, delta={}",
                cell.heterodoxy_bin.value(),
                cell.age_bin.value(),
                current,
                delta
            ),
        }
    }
}

impl std::error::Error for HistogramError {}

/// index in histogram representing heterodoxy bin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeterodoxyBin(usize);
/// index in histogram representing age bin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgeBin(usize);
/// value of histogram hist[][], count of people who fall into that combination of row / col
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count(u64);

/// actual cell in histogram, that way we can access ergonomically
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistogramCell {
    pub heterodoxy_bin: HeterodoxyBin,
    pub age_bin: AgeBin,
}

impl HeterodoxyBin {
    pub fn value(&self) -> usize {
        self.0
    }
}

impl From<usize> for HeterodoxyBin {
    fn from(val: usize) -> Self {
        HeterodoxyBin(val)
    }
}

impl AgeBin {
    pub fn value(&self) -> usize {
        self.0
    }
}

impl From<usize> for AgeBin {
    fn from(val: usize) -> Self {
        AgeBin(val)
    }
}

impl Count {
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// histogram struct. contains the actualy nested vec, but has constructors / accessors for it so that I am not trying to just use a nested vec raw
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationHistogram {
    counts: Vec<Vec<Count>>,
}

impl PopulationHistogram {
    pub fn bin(
        &mut self,
        heterodoxy: f64,
        age: usize,
        num_heterodoxy_bins: usize,
        num_age_bins: usize,
    ) -> Result<(), HistogramError> {
        let nearest_het_bin = (heterodoxy * num_heterodoxy_bins as f64)
            .round()
            .to_usize()
            .unwrap(); // cause we know we round, and it's bounded

        let nearest_age_bin = (age as f64 * num_age_bins as f64)
            .round()
            .to_usize()
            .unwrap(); // cause we know we round, and it's bounded

        self.get_mut_count(HistogramCell {
            heterodoxy_bin: nearest_het_bin.into(),
            age_bin: nearest_age_bin.into(),
        })?
        .increment();
        Ok(())
    }

    /// Sums all counts across every heterodoxy bin and age bin to return the total population
    pub fn total(&self) -> u64 {
        self.counts
            .iter()
            .flat_map(|row| row.iter())
            .map(|count| count.value())
            .sum()
    }

    pub fn new(num_heterodoxy_bins: usize, num_age_bins: usize) -> Self {
        PopulationHistogram {
            counts: vec![vec![Count(0); num_heterodoxy_bins + 1]; num_age_bins + 1],
        }
    }

    pub fn get_count(&self, cell: HistogramCell) -> Result<&Count, HistogramError> {
        self.counts
            .get(cell.age_bin.value())
            .and_then(|row| row.get(cell.heterodoxy_bin.value()))
            .ok_or(HistogramError::CellNotFound(cell))
    }

    pub fn get_mut_count(&mut self, cell: HistogramCell) -> Result<&mut Count, HistogramError> {
        self.counts
            .get_mut(cell.age_bin.value())
            .and_then(|row| row.get_mut(cell.heterodoxy_bin.value()))
            .ok_or(HistogramError::CellNotFound(cell))
    }

    /// All heterodoxy bins for a given age band.
    pub fn row(&self, age_bin: AgeBin) -> Result<&[Count], HistogramError> {
        self.counts
            .get(age_bin.value())
            .map(Vec::as_slice)
            .ok_or_else(|| HistogramError::CellNotFound(HistogramCell {
                age_bin,
                heterodoxy_bin: HeterodoxyBin(0),
            }))
    }

    pub fn row_mut(&mut self, age_bin: AgeBin) -> Result<&mut [Count], HistogramError> {
        self.counts
            .get_mut(age_bin.value())
            .map(Vec::as_mut_slice)
            .ok_or_else(|| HistogramError::CellNotFound(HistogramCell {
                age_bin,
                heterodoxy_bin: HeterodoxyBin(0),
            }))
    }

    /// All age bins for a given heterodoxy bin.
    pub fn col(&self, het_bin: HeterodoxyBin) -> Result<impl Iterator<Item = &Count>, HistogramError> {
        let idx = het_bin.value();
        if self.counts.first().map_or(true, |row| idx >= row.len()) {
            return Err(HistogramError::CellNotFound(HistogramCell {
                age_bin: AgeBin(0),
                heterodoxy_bin: het_bin,
            }));
        }
        Ok(self.counts.iter().map(move |row| &row[idx]))
    }

    pub fn col_mut(&mut self, het_bin: HeterodoxyBin) -> Result<impl Iterator<Item = &mut Count> + '_, HistogramError> {
        let idx = het_bin.value();
        if self.counts.first().map_or(true, |row| idx >= row.len()) {
            return Err(HistogramError::CellNotFound(HistogramCell {
                age_bin: AgeBin(0),
                heterodoxy_bin: het_bin,
            }));
        }
        Ok(self.counts.iter_mut().map(move |row| &mut row[idx]))
    }

    pub fn adjust(&mut self, cell: HistogramCell, delta: i64) -> Result<i64, HistogramError> {
        let count = self.get_mut_count(cell)?;
        let new_val = count.0 as i64 + delta;
        if new_val < 0 {
            return Err(HistogramError::WouldGoNegative {
                cell,
                current: count.0,
                delta,
            });
        }
        count.0 = new_val as u64;
        Ok(delta)
    }
}

impl std::fmt::Display for PopulationHistogram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, row) in self.counts.iter().enumerate() {
            write!(f, "AgeBin {}: [", i)?;
            for (j, count) in row.iter().enumerate() {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "HeterodoxyBin {}: {}", j, count.value())?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}
