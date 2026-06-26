use thiserror::Error;

#[derive(Debug, Error)]
pub enum HistogramError {
    #[error("adjustment would go negative: age_band={}, het_bin={}, current={current}, delta={delta}", age_band.0, het_bin.0)]
    WouldGoNegative {
        age_band: AgeBand,
        het_bin: HeterodoxyBin,
        current: u64,
        delta: i64,
    },
    #[error("count adjustment would go negative: current={current}, delta={delta}")]
    Underflow { current: u64, delta: i64 },
    #[error("out of bounds: age_band={}, het_bin={}", age_band.0, het_bin.0)]
    OutOfBounds {
        age_band: AgeBand,
        het_bin: HeterodoxyBin,
    },
}

/// index in histogram representing heterodoxy bin
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeterodoxyBin(usize);

/// index in histogram representing age band
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgeBand(usize);

/// value of a histogram cell: count of people in that (age_band, het_bin) combination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count(pub u64);

impl HeterodoxyBin {
    pub fn value(&self) -> usize {
        self.0
    }

    pub fn to_heterodoxy(&self, num_het_bins: usize) -> f64 {
        self.0 as f64 / num_het_bins as f64
    }

    pub fn from_heterodoxy(het: f64, num_het_bins: usize) -> Self {
        HeterodoxyBin((het * num_het_bins as f64).round() as usize)
    }
}

impl From<usize> for HeterodoxyBin {
    fn from(val: usize) -> Self {
        HeterodoxyBin(val)
    }
}

impl AgeBand {
    pub fn value(&self) -> usize {
        self.0
    }

    pub fn get_age(&self, num_age_bins: usize, max_age: u8) -> usize {
        self.value() * max_age as usize / num_age_bins
    }

    pub fn from_age(age: usize, num_age_bins: usize, max_age: u8) -> Self {
        AgeBand(
            ((age as f64 * (num_age_bins as f64 / max_age as f64)).round() as usize)
                .min(num_age_bins - 1),
        )
    }
}

impl From<usize> for AgeBand {
    fn from(val: usize) -> Self {
        AgeBand(val)
    }
}

impl Count {
    pub fn empty(&mut self) {
        self.0 = 0;
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn adjust(&mut self, delta: i64) -> Result<i64, HistogramError> {
        let new_val = self.0 as i64 + delta;
        if new_val < 0 {
            return Err(HistogramError::Underflow {
                current: self.0,
                delta,
            });
        }
        self.0 = new_val as u64;
        Ok(delta)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// histogram of population counts indexed by [age_band][het_bin]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationHistogram {
    counts: Vec<Vec<Count>>,
}

// --- indexing ---

impl PopulationHistogram {
    pub fn get(&self, age: AgeBand, het: HeterodoxyBin) -> Option<&Count> {
        self.counts.get(age.0)?.get(het.0)
    }

    pub fn get_mut(&mut self, age: AgeBand, het: HeterodoxyBin) -> Option<&mut Count> {
        self.counts.get_mut(age.0)?.get_mut(het.0)
    }

    pub fn get_age_band(&self, age: AgeBand) -> Option<&[Count]> {
        self.counts.get(age.0).map(Vec::as_slice)
    }
}

// --- core methods ---

impl PopulationHistogram {
    pub fn new(num_het_bins: usize, num_age_bands: usize) -> Self {
        PopulationHistogram {
            counts: vec![vec![Count(0); num_het_bins + 1]; num_age_bands],
        }
    }

    pub fn take_counts(&mut self) -> Vec<Vec<Count>> {
        std::mem::take(&mut self.counts)
    }

    pub fn swap_counts(&mut self, new_counts: Vec<Vec<Count>>) {
        self.counts = new_counts;
    }

    pub fn bin(
        &mut self,
        heterodoxy: f64,
        age: usize,
        num_het_bins: usize,
        num_age_bands: usize,
        max_age: u8,
    ) -> Result<(), HistogramError> {
        let het_bin = HeterodoxyBin::from_heterodoxy(heterodoxy, num_het_bins);

        // ages at the very top of the range (e.g. one below max_age) round up to
        // band index `num_age_bands`, which is one past the last row — clamp them
        // into the oldest band so binning the initial population can't panic.
        let age_band = AgeBand::from_age(age, num_age_bands, max_age);

        self.get_mut(age_band, het_bin)
            .ok_or(HistogramError::OutOfBounds { age_band, het_bin })?
            .increment();
        Ok(())
    }

    pub fn total(&self) -> u64 {
        self.counts
            .iter()
            .flat_map(|row| row.iter())
            .map(|c| c.value())
            .sum()
    }

    /// Drop all count data for a religion that has gone extinct; replaces the
    /// backing allocation with an empty vec so the memory is freed immediately.
    /// Only call this when a religion dies — the histogram is unusable after.
    pub fn clear(&mut self) {
        self.counts = vec![];
    }

    pub fn total_in_age_band(&self, age_band: AgeBand) -> Option<u64> {
        Some(self.get_age_band(age_band)?.iter().map(|c| c.value()).sum())
    }

    /// weighted mean heterodoxy across all age bands, using bin index / num_het_bins as
    /// each bin's representative heterodoxy value. returns 0.0 for an empty histogram.
    /// weighted mean heterodoxy for only the bins at or above `threshold`.
    /// returns 0.0 when no population exists above the threshold.
    pub fn mean_heterodoxy_above(&self, threshold: HeterodoxyBin) -> f64 {
        let num_het_bins = match self.counts.first() {
            Some(row) if !row.is_empty() => row.len() - 1,
            _ => return 0.0,
        };
        if num_het_bins == 0 {
            return 0.0;
        }

        let mut weighted_sum = 0.0f64;
        let mut total = 0u64;

        for row in &self.counts {
            for (het_bin_idx, count) in row.iter().enumerate().skip(threshold.0) {
                let pop = count.value();
                weighted_sum += HeterodoxyBin(het_bin_idx).to_heterodoxy(num_het_bins) * pop as f64;
                total += pop;
            }
        }

        if total == 0 {
            0.0
        } else {
            weighted_sum / total as f64
        }
    }

    pub fn mean_heterodoxy(&self) -> f64 {
        let num_het_bins = match self.counts.first() {
            Some(row) if !row.is_empty() => row.len() - 1,
            _ => return 0.0,
        };
        if num_het_bins == 0 {
            return 0.0;
        }

        let mut weighted_sum = 0.0f64;
        let mut total = 0u64;

        for row in &self.counts {
            for (het_bin_idx, count) in row.iter().enumerate() {
                let pop = count.value();
                weighted_sum += HeterodoxyBin(het_bin_idx).to_heterodoxy(num_het_bins) * pop as f64;
                total += pop;
            }
        }

        if total == 0 {
            0.0
        } else {
            weighted_sum / total as f64
        }
    }

    pub fn adjust(
        &mut self,
        age_band: AgeBand,
        het_bin: HeterodoxyBin,
        delta: i64,
    ) -> Result<i64, HistogramError> {
        let count = self
            .get_mut(age_band, het_bin)
            .ok_or(HistogramError::OutOfBounds { age_band, het_bin })?;
        let new_val = count.0 as i64 + delta;
        if new_val < 0 {
            return Err(HistogramError::WouldGoNegative {
                age_band,
                het_bin,
                current: count.0,
                delta,
            });
        }
        count.0 = new_val as u64;
        Ok(delta)
    }
}

// --- iteration ---

impl PopulationHistogram {
    /// Yields (AgeBand, Iterator<(HeterodoxyBin, &Count)>) for each age band.
    pub fn iter_bands(
        &self,
    ) -> impl Iterator<Item = (AgeBand, impl Iterator<Item = (HeterodoxyBin, &Count)>)> {
        self.counts.iter().enumerate().map(|(i, row)| {
            (
                AgeBand(i),
                row.iter().enumerate().map(|(j, c)| (HeterodoxyBin(j), c)),
            )
        })
    }

    /// Yields (AgeBand, Iterator<(HeterodoxyBin, &mut Count)>) for each age band.
    pub fn iter_bands_mut(
        &mut self,
    ) -> impl Iterator<Item = (AgeBand, impl Iterator<Item = (HeterodoxyBin, &mut Count)>)> {
        self.counts.iter_mut().enumerate().map(|(i, row)| {
            (
                AgeBand(i),
                row.iter_mut()
                    .enumerate()
                    .map(|(j, c)| (HeterodoxyBin(j), c)),
            )
        })
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Vec<Count>> {
        self.counts.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Vec<Count>> {
        self.counts.iter_mut()
    }
}

impl IntoIterator for PopulationHistogram {
    type Item = Vec<Count>;
    type IntoIter = std::vec::IntoIter<Vec<Count>>;

    fn into_iter(self) -> Self::IntoIter {
        self.counts.into_iter()
    }
}

impl<'a> IntoIterator for &'a PopulationHistogram {
    type Item = &'a Vec<Count>;
    type IntoIter = std::slice::Iter<'a, Vec<Count>>;

    fn into_iter(self) -> Self::IntoIter {
        self.counts.iter()
    }
}

impl<'a> IntoIterator for &'a mut PopulationHistogram {
    type Item = &'a mut Vec<Count>;
    type IntoIter = std::slice::IterMut<'a, Vec<Count>>;

    fn into_iter(self) -> Self::IntoIter {
        self.counts.iter_mut()
    }
}

impl std::fmt::Display for PopulationHistogram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (age_band, row) in self.counts.iter().enumerate() {
            write!(f, "AgeBand {}: [", age_band)?;
            for (het_bin, count) in row.iter().enumerate() {
                if het_bin > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "HeterodoxyBin {}: {}", het_bin, count.value())?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}
