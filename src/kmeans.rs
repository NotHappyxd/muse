use image::{DynamicImage, GenericImageView};

const QUANTIZATION_LEVEL: usize = 5;
const BIN_COUNT: usize = 1 << (QUANTIZATION_LEVEL * 3);

#[derive(Default, Clone)]
pub struct ColorBin {
    r_sum: u64,
    g_sum: u64,
    b_sum: u64,
    count: usize,
}

#[derive(Debug)]
pub struct Cluster {
    pub color: Color,
    size: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {

    pub fn zero() -> Color {
        Color { r: 0.0, g: 0.0, b: 0.0 }
    }
    #[inline]
    pub fn distance(&self, other: &Color) -> f32 {
        let delta_r = self.r - other.r;
        let delta_g = self.g - other.g;
        let delta_b = self.b - other.b;

        delta_r * delta_r + delta_g * delta_g + delta_b * delta_b
    }
}

pub fn quantize_pixel(r: u8, g: u8, b: u8) -> usize {
    const SHIFT: u8 = 8 - QUANTIZATION_LEVEL as u8;
    let red = r >> SHIFT;
    let green = g >> SHIFT;
    let blue = b >> SHIFT;

    ((red as usize) << (QUANTIZATION_LEVEL << 1))
        | ((green as usize) << QUANTIZATION_LEVEL)
        | (blue as usize)
}

pub fn color_histogram(img: &DynamicImage) -> Vec<(Color, usize)> {
    let mut bins: Vec<ColorBin> = vec![ColorBin::default(); BIN_COUNT];

    for (_, _, pixel) in img.pixels() {
        let rgb = pixel.0;

        let idx = quantize_pixel(rgb[0], rgb[1], rgb[2]);

        let bin = &mut bins[idx];

        bin.r_sum += rgb[0] as u64;
        bin.g_sum += rgb[1] as u64;
        bin.b_sum += rgb[2] as u64;
        bin.count += 1;
    }

    bins.into_iter()
        .filter(|b| b.count > 0)
        .map(|b| {
            (
                Color {
                    r: b.r_sum as f32 / b.count as f32,
                    g: b.g_sum as f32 / b.count as f32,
                    b: b.b_sum as f32 / b.count as f32,
                },
                b.count,
            )
        })
        .collect()
}

pub(crate) fn kmeans_recode(
    histogram: &[(Color, usize)],
    k: usize,
    max_iterations: usize,
) -> Vec<Cluster> {
    let mut centroids: Vec<Cluster> = Vec::with_capacity(k);

    for _ in 0..k {
        centroids.push(Cluster {
            color: histogram[fastrand::usize(0..histogram.len())].0,
            size: 0
        });
    }

    let mut sums = vec![Color::zero(); k];
    let mut counts = vec![0usize; k];

    for iteration in 0..max_iterations {
        let mut total_movement: f32 = 0.0;
        sums.fill(Color::zero());
        counts.fill(0);

        for (color, count) in histogram {
            let weight = *count as f32;
            let (mut smallest_distance, mut index) = (color.distance(&centroids[0].color), 0);

            for centroid in 1..centroids.len() {
                let distance = color.distance(&centroids[centroid].color);

                if distance < smallest_distance {
                    smallest_distance = distance;
                    index = centroid;
                }
            }

            sums[index].r += color.r * weight;
            sums[index].g += color.g * weight;
            sums[index].b += color.b * weight;
            counts[index] += *count;
        }

        for i in 0..k {
            if counts[i] > 0 {
                let color = Color {
                    r: sums[i].r / counts[i] as f32,
                    g: sums[i].g / counts[i] as f32,
                    b: sums[i].b / counts[i] as f32,
                };
                let distance = color.distance(&centroids[i].color);
                total_movement += distance;
                centroids[i].color = color;

                centroids[i].size = counts[i];
            }else if counts[i] == 0 {
                centroids[i].size = 0;
                continue;
            }
        }

        if (total_movement / k as f32) < 0.02 { // Converges
            break
        }
    }

    centroids.sort_by_key(|c| std::cmp::Reverse(c.size));

    centroids
}
