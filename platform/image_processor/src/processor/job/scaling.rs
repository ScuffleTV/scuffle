use std::ops::MulAssign;

#[derive(Debug, Clone)]
pub struct ScalingOptions {
    pub input_width: usize,
    pub input_height: usize,
    pub aspect_ratio: Ratio,
    pub clamp_aspect_ratio: bool,
    pub preserve_aspect_width: bool,
    pub preserve_aspect_height: bool,
    pub upscale: Upscale,
    pub input_image_scaling: bool,
    pub scales: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upscale {
    Yes,
    No,
    NoPreserveSource,
}

impl From<pb::scuffle::platform::internal::image_processor::task::Upscale> for Upscale {
    fn from(value: pb::scuffle::platform::internal::image_processor::task::Upscale) -> Self {
        match value {
            pb::scuffle::platform::internal::image_processor::task::Upscale::Yes => Upscale::Yes,
            pb::scuffle::platform::internal::image_processor::task::Upscale::No => Upscale::No,
            pb::scuffle::platform::internal::image_processor::task::Upscale::NoPreserveSource => Upscale::NoPreserveSource,
        }
    }
}

impl Upscale {
    pub fn is_yes(&self) -> bool {
        matches!(self, Upscale::Yes)
    }

    pub fn is_no(&self) -> bool {
        matches!(self, Upscale::No | Upscale::NoPreserveSource)
    }

    pub fn preserve_source(&self) -> bool {
        matches!(self, Upscale::NoPreserveSource)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

#[derive(Debug, Clone, Copy)]
pub struct Ratio {
    n: usize,
    d: usize,
}

impl Ratio {
    pub const ONE: Self = Self::new(1, 1);

    pub const fn new(n: usize, d: usize) -> Self {
        Self { n, d }.simplify()
    }

    const fn gcd(&self) -> usize {
        let mut a = self.n;
        let mut b = self.d;

        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }

        a
    }

    const fn simplify(mut self) -> Self {
        let gcd = self.gcd();

        self.n /= gcd;
        self.d /= gcd;

        self
    }

    fn as_f64(&self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

impl std::ops::Div<usize> for Ratio {
    type Output = Ratio;

    fn div(self, rhs: usize) -> Self::Output {
        Self {
            n: self.n,
            d: self.d * rhs,
        }.simplify()
    }
}

impl std::ops::Mul<usize> for Ratio {
    type Output = Ratio;

    fn mul(self, rhs: usize) -> Self::Output {
        Self {
            n: self.n * rhs,
            d: self.d,
        }.simplify()
    }
}

impl std::ops::Div<Ratio> for Ratio {
    type Output = Ratio;

    fn div(self, rhs: Ratio) -> Self::Output {
        Self {
            n: self.n * rhs.d,
            d: self.d * rhs.n,
        }.simplify()
    }
}

impl std::ops::Mul<Ratio> for Ratio {
    type Output = Ratio;

    fn mul(self, rhs: Ratio) -> Self::Output {
        Self {
            n: self.n * rhs.n,
            d: self.d * rhs.d,
        }.simplify()
    }
}

impl PartialEq for Ratio {
    fn eq(&self, other: &Self) -> bool {
        let this = self.simplify();
        let other = other.simplify();

        this.n == other.n && this.d == other.d
    }
}

impl Eq for Ratio {}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let this = self.simplify();
        let other = other.simplify();

        Some((this.n * other.d).cmp(&(this.d * other.n)))
    }
}

impl Ord for Ratio {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let this = self.simplify();
        let other = other.simplify();

        (this.n * other.d).cmp(&(this.d * other.n))
    }
}

impl MulAssign for Ratio {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::ops::DivAssign for Ratio {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<T> std::ops::Div<T> for Size<T>
where
    T: std::ops::Div<Output = T> + Copy,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

impl<T> std::ops::Mul<T> for Size<T>
where
    T: std::ops::Mul<Output = T> + Copy,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl ScalingOptions {
    pub fn compute(&mut self) -> Vec<Size<usize>> {
        // Sorts the scales from smallest to largest.
        self.scales.sort_by(|a, b| a.partial_cmp(&b).unwrap());

        let mut scales = self.compute_scales();
        let padded_size = self.padded_size();

        let (best_idx, input_scale_factor) = scales.iter().position(|(size, _)| {
            size.width >= padded_size.width || size.height >= padded_size.height
        }).map(|idx| (idx, Ratio::ONE)).unwrap_or_else(|| {
            let size = scales.last().unwrap().0;

            // Since its the padded size, the aspect ratio is the same as the target aspect ratio.
            let input_scale_factor = padded_size.width / size.width;

            (scales.len() - 1, input_scale_factor)
        });

        dbg!(&scales);

        if self.input_image_scaling {
            let scaled_width = padded_size.width / scales[best_idx].1 / input_scale_factor;
            let scaled_height = padded_size.height / scales[best_idx].1 / input_scale_factor;
            scales.iter_mut().for_each(|(size, scale)| {
                size.width = *scale * scaled_width;
                size.height = *scale * scaled_height;
            });
        };


        if self.upscale.preserve_source() {
            let padded_size = padded_size / input_scale_factor;

            dbg!(&padded_size);

            let size = scales[best_idx].0;

            if size.width > padded_size.width || size.height > padded_size.height {
                scales[best_idx].0 = padded_size;
            }
        }

        if self.clamp_aspect_ratio {
            scales.iter_mut().for_each(|(size, scale)| {
                let scale = *scale;
    
                if self.aspect_ratio < Ratio::ONE && size.height > scale / self.aspect_ratio {
                    let height = scale / self.aspect_ratio;
                    size.width *= height / size.height;
                    size.height = height;
                } else if self.aspect_ratio > Ratio::ONE && size.width > scale * self.aspect_ratio {
                    let width = scale * self.aspect_ratio;
                    size.height *= width / size.width;
                    size.width = width;
                } else if self.aspect_ratio == Ratio::ONE && size.width > scale {
                    size.height *= scale / size.width;
                    size.width = scale;
                } else if self.aspect_ratio == Ratio::ONE && size.height > scale {
                    size.width *= scale / size.height;
                    size.height = scale;
                }
    
                size.width = size.width.max(Ratio::ONE);
                size.height = size.height.max(Ratio::ONE);
            });
        }

        if self.upscale.is_no() {
            scales.retain(|(size, _)| {
                size.width <= padded_size.width && size.height <= padded_size.height
            });
        }

        scales.into_iter().map(|(mut size, _)| {
            let input_aspect_ratio = self.input_aspect_ratio();

            if self.preserve_aspect_height && self.aspect_ratio <= Ratio::ONE {
                let height = size.height * self.aspect_ratio / input_aspect_ratio;
                // size.width *= size.height / height;
                size.height = height;
            } else if self.preserve_aspect_width && self.aspect_ratio >= Ratio::ONE {
                let width = size.width * input_aspect_ratio / self.aspect_ratio;
                // size.height *= size.width / width;
                size.width = width;
            }

            Size {
                width: size.width.as_f64().round() as usize,
                height: size.height.as_f64().round() as usize,
            }
        }).collect()
    }

    fn compute_scales(&self) -> Vec<(Size<Ratio>, Ratio)> {
        self.scales.iter().copied().map(|scale| {
            let scale = Ratio::new(scale, 1);

            let (width, height) = if self.aspect_ratio > Ratio::ONE {
                (scale * self.aspect_ratio, scale)
            } else {
                (scale, scale / self.aspect_ratio)
            };

            (Size { width, height }, scale)
        }).collect()
    }

    fn input_aspect_ratio(&self) -> Ratio {
        Ratio {
            n: self.input_width,
            d: self.input_height,
        }.simplify()
    }

    fn padded_size(&self) -> Size<Ratio> {
        let width = Ratio::new(self.input_width, 1);
        let height = Ratio::new(self.input_height, 1);

        let (width, height) = if self.aspect_ratio < Ratio::ONE {
            (width, width / self.aspect_ratio)
        } else {
            (height * self.aspect_ratio, height)
        };

        Size { width, height }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_scales_same_aspect() {
        let mut options = ScalingOptions {
            input_width: 100,
            input_height: 100,
            aspect_ratio: Ratio::new(1, 1),
            preserve_aspect_width: false,
            preserve_aspect_height: false,
            upscale: Upscale::Yes,
            input_image_scaling: false,
            clamp_aspect_ratio: true,
            scales: vec![
                32,
                64,
                96,
                128,
            ],
        };

        assert_eq!(options.compute(), vec![
            Size { width: 32, height: 32 },
            Size { width: 64, height: 64 },
            Size { width: 96, height: 96 },
            Size { width: 128, height: 128 },
        ]);

        options.upscale = Upscale::No;

        assert_eq!(options.compute(), vec![
            Size { width: 32, height: 32 },
            Size { width: 64, height: 64 },
            Size { width: 96, height: 96 },
        ]);

        options.upscale = Upscale::NoPreserveSource;

        assert_eq!(options.compute(), vec![
            Size { width: 32, height: 32 },
            Size { width: 64, height: 64 },
            Size { width: 96, height: 96 },
            Size { width: 100, height: 100 },
        ]);

        options.input_height = 112;
        options.input_width = 112;
        options.input_image_scaling = true;
        options.upscale = Upscale::No;

        assert_eq!(options.compute(), vec![
            Size { width: 28, height: 28 },
            Size { width: 56, height: 56 },
            Size { width: 84, height: 84 },
            Size { width: 112, height: 112 },
        ]);
    }

    #[test]
    fn test_compute_scales_different_aspect() {
        let mut options = ScalingOptions {
            input_width: 100,
            input_height: 100,
            aspect_ratio: Ratio::new(16, 9),
            preserve_aspect_width: false,
            preserve_aspect_height: false,
            upscale: Upscale::Yes,
            input_image_scaling: false,
            clamp_aspect_ratio: true,
            scales: vec![
                360,
                720,
                1080,
            ],
        };

        assert_eq!(options.compute(), vec![
            Size { width: 640, height: 360 },
            Size { width: 1280, height: 720 },
            Size { width: 1920, height: 1080 },
        ]);

        options.upscale = Upscale::No;
        assert_eq!(options.compute(), vec![]);

        options.upscale = Upscale::NoPreserveSource;
        assert_eq!(options.compute(), vec![
            Size { width: 178, height: 100 },
        ]);

        options.aspect_ratio = Ratio::new(9, 16);
        options.upscale = Upscale::Yes;

        assert_eq!(options.compute(), vec![
            Size { width: 360, height: 640 },
            Size { width: 720, height: 1280 },
            Size { width: 1080, height: 1920 },
        ]);

        options.upscale = Upscale::No;
        assert_eq!(options.compute(), vec![]);

        options.upscale = Upscale::NoPreserveSource;
        assert_eq!(options.compute(), vec![
            Size { width: 100, height: 178 },
        ]);

        options.input_width = 1920;
        options.input_height = 1080;
        options.upscale = Upscale::Yes;

        assert_eq!(options.compute(), vec![
            Size { width: 360, height: 640 },
            Size { width: 720, height: 1280 },
            Size { width: 1080, height: 1920 },
        ]);

        options.upscale = Upscale::No;
        assert_eq!(options.compute(), vec![
            Size { width: 360, height: 640 },
            Size { width: 720, height: 1280 },
            Size { width: 1080, height: 1920 },
        ]);

        options.upscale = Upscale::NoPreserveSource;
        assert_eq!(options.compute(), vec![
            Size { width: 360, height: 640 },
            Size { width: 720, height: 1280 },
            Size { width: 1080, height: 1920 },
        ]);
    }

    #[test]
    fn test_compute_scales_image_scaling() {
        let mut options = ScalingOptions {
            input_width: 112,
            input_height: 112,
            aspect_ratio: Ratio::new(3, 1),
            preserve_aspect_width: true,
            preserve_aspect_height: true,
            upscale: Upscale::NoPreserveSource,
            input_image_scaling: true,
            clamp_aspect_ratio: true,
            scales: vec![
                32,
                64,
                96,
                128,
            ],
        };

        assert_eq!(options.compute(), vec![
            Size { width: 28, height: 28 },
            Size { width: 56, height: 56 },
            Size { width: 84, height: 84 },
            Size { width: 112, height: 112 },
        ]);

        options.input_width = 112 * 2;
        assert_eq!(options.compute(), vec![
            Size { width: 28 * 2, height: 28 },
            Size { width: 56 * 2, height: 56 },
            Size { width: 84 * 2, height: 84 },
            Size { width: 112 * 2, height: 112 },
        ]);

        options.input_width = 112 * 3;
        assert_eq!(options.compute(), vec![
            Size { width: 28 * 3, height: 28 },
            Size { width: 56 * 3, height: 56 },
            Size { width: 84 * 3, height: 84 },
            Size { width: 112 * 3, height: 112 },
        ]);

        options.input_width = 112 * 4;
        assert_eq!(options.compute(), vec![
            Size { width: 32 * 3, height: 24 },
            Size { width: 64 * 3, height: 48 },
            Size { width: 96 * 3, height: 72 },
            Size { width: 128 * 3, height: 96 },
        ]);

        options.input_width = 112 / 2;
        assert_eq!(options.compute(), vec![
            Size { width: 28 / 2, height: 28 },
            Size { width: 56 / 2, height: 56 },
            Size { width: 84 / 2, height: 84 },
            Size { width: 112 / 2, height: 112 },
        ]);

        options.input_width = 112 / 3;
        assert_eq!(options.compute(), vec![
            Size { width: 9, height: 28 },
            Size { width: 19, height: 56 },
            Size { width: 28, height: 84 },
            Size { width: 37, height: 112 },
        ]);
    }

    #[test]
    fn test_compute_scales_any_scale() {
        let mut options = ScalingOptions {
            input_width: 245,
            input_height: 1239,
            aspect_ratio: Ratio::new(1, 1),
            preserve_aspect_width: true,
            preserve_aspect_height: true,
            upscale: Upscale::NoPreserveSource,
            input_image_scaling: true,
            clamp_aspect_ratio: true,
            scales: vec![
                720,
                1080,
            ],
        };

        assert_eq!(options.compute(), vec![
            Size { width: 142, height: 720 },
            Size { width: 214, height: 1080 },
        ]);

        options.input_width = 1239;
        options.input_height = 245;

        assert_eq!(options.compute(), vec![
            Size { width: 720, height: 142 },
            Size { width: 1080, height: 214 },
        ]);

        options.clamp_aspect_ratio = false;
        options.input_image_scaling = false;
        options.input_height = 1239;
        options.input_width = 245;

        assert_eq!(options.compute(), vec![
            Size { width: 142, height: 720 },
            Size { width: 214, height: 1080 },
        ]);

        options.input_height = 245;
        options.input_width = 1239;

        assert_eq!(options.compute(), vec![
            Size { width: 720, height: 142 },
            Size { width: 1080, height: 214 },
        ]);
    }
}
