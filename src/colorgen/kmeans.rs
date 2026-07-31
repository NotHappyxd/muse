use image::{DynamicImage, GenericImageView};
use crate::colorgen::colors::Lab;
use crate::colorgen::conversions;

const QUANTIZATION_LEVEL: usize = 5;
const BIN_COUNT: usize = 1 << (QUANTIZATION_LEVEL * 3);

#[derive(Default, Clone)]
pub struct ColorBin {
    l_sum: f32,
    a_sum: f32,
    b_sum: f32,
    weight: f32,
}

#[derive(Debug)]
pub struct Cluster {
    pub(crate) color: Lab,
    pub(crate) size: f32,
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

pub fn color_histogram(img: &DynamicImage) -> Vec<(Lab, f32)> {
    let mut bins: Vec<ColorBin> = vec![ColorBin::default(); BIN_COUNT];

    for (x, y, pixel) in img.pixels() {
        let rgb = pixel.0;

        let idx = quantize_pixel(rgb[0], rgb[1], rgb[2]);
        let bin = &mut bins[idx];
        let lab = conversions::rgb_to_oklab([rgb[0], rgb[1], rgb[2]]);

        let dx = (x as f32 + 0.5) / img.width() as f32 - 0.5;
        let dy = (y as f32 + 0.5) / img.height() as f32 - 0.5;

        let dist2 = dx * dx + dy * dy;

        let weight = 0.15 + 0.85 * (-dist2 * 18.0).exp();

        bin.l_sum += lab.l * weight;
        bin.a_sum += lab.a * weight;
        bin.b_sum += lab.b * weight;
        bin.weight += weight;
    }

    bins.into_iter()
        .filter(|b| b.weight > 0.0)
        .map(|b| {
            (Lab {
                l: b.l_sum / b.weight,
                a: b.a_sum / b.weight,
                b: b.b_sum / b.weight,
            }, b.weight)
        })
        .collect()
}

pub(crate) fn kmeans(
    histogram: &[(Lab, f32)],
    k: usize,
    max_iterations: usize,
) -> Vec<Cluster> {
    let mut centroids: Vec<Cluster> = select_centroids(histogram, k);

    let mut sums = vec![Lab::default(); k];
    let mut weights = vec![0.0; k];

    for _iteration in 0..max_iterations {
        let mut highest_movement: f32 = 0.0;
        sums.fill(Lab::default());
        weights.fill(0.0);

        for (color, count) in histogram {
            let weight = count;
            let (mut smallest_distance, mut index) = (color.distance(&centroids[0].color), 0);

            for (idx, centroid) in centroids.iter().enumerate().skip(1) {
                let distance = color.distance(&centroid.color);

                if distance < smallest_distance {
                    smallest_distance = distance;
                    index = idx;
                }
            }

            sums[index].l += color.l * weight;
            sums[index].a += color.a * weight;
            sums[index].b += color.b * weight;
            weights[index] += *count;
        }

        for i in 0..k {
            if weights[i] > 0.0 {
                let color = Lab {
                    l: sums[i].l / weights[i],
                    a: sums[i].a / weights[i],
                    b: sums[i].b / weights[i],
                };

                let movement = color.distance(&centroids[i].color);

                if movement > highest_movement {
                    highest_movement = movement;
                }

                centroids[i].color = color;
                centroids[i].size = weights[i];
            }else {
                centroids[i].size = 0.0;
            }
        }

        if highest_movement < 0.000004 {
            break
        }
    }

    centroids.retain(|c| c.size > 0.0);
    centroids.sort_by(|a, b| b.size.partial_cmp(&a.size).unwrap());


    centroids
}

pub fn select_centroids(histogram: &[(Lab, f32)], k: usize) -> Vec<Cluster> {
    let mut centroids: Vec<Cluster> = Vec::with_capacity(k);
    centroids.push(Cluster {
        color: histogram[fastrand::usize(0..histogram.len())].0,
        size: 0.0,
    });

    let mut min_d2: Vec<f32> = histogram
        .iter()
        .map(|(color, _)| color.distance(&centroids[0].color))
        .collect();

    for _ in 1..k {
        let total: f32 = min_d2.iter().zip(histogram).map(|(d2, (_, c))| d2 * c).sum();
        let mut threshold = fastrand::f32() * total;
        let mut chosen = histogram.len() - 1;
        for (i, (d2, (_, c))) in min_d2.iter().zip(histogram).enumerate() {
            threshold -= d2 * c;
            if threshold <= 0.0 {
                chosen = i;
                break;
            }
        }

        let new_centroid = histogram[chosen].0;
        centroids.push(Cluster { color: new_centroid, size: 0.0 });

        for (d2, (color, _)) in min_d2.iter_mut().zip(histogram) {
            let d = color.distance(&new_centroid);
            if d < *d2 {
                *d2 = d;
            }
        }
    }

    centroids
}