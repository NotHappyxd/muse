use std::array;
use crate::colorgen::kmeans::Lab;
use std::sync::{LazyLock, OnceLock};

static LINEAR_CACHE: LazyLock<[f32; 256]> = LazyLock::new(|| {
    array::from_fn(|i| {
        let v = i as f32 / 255.0;
        if v > 0.04045 {
            ((v + 0.055) / 1.055).powf(2.4)
        } else {
            v / 12.92
        }
    })
});

pub fn rgb_to_oklab(rgb: [u8; 3]) -> Lab {
    let arr = rgb_to_lab_arr(rgb);

    Lab {
        l: arr[0],
        a: arr[1],
        b: arr[2],
    }
}

pub fn rgb_to_lab_arr(rgb: [u8; 3]) -> [f32; 3] {
    let linear_r = LINEAR_CACHE[rgb[0] as usize];
    let linear_g = LINEAR_CACHE[rgb[1] as usize];
    let linear_b = LINEAR_CACHE[rgb[2] as usize];

    let mut l = 0.4122214708 * linear_r + 0.5363325363 * linear_g + 0.0514459929 * linear_b;
    let mut m = 0.2119034982 * linear_r + 0.6806995451 * linear_g + 0.1073969566 * linear_b;
    let mut s = 0.0883024619 * linear_r + 0.2817188376 * linear_g + 0.6299787005 * linear_b;

    l = l.cbrt();
    m = m.cbrt();
    s = s.cbrt();

    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn oklab_to_linear_srgb(lab: &Lab) -> [f32; 3] {
    let l_ = lab.l + 0.3963377774 * lab.a + 0.2158037573 * lab.b;
    let m_ = lab.l - 0.1055613458 * lab.a - 0.0638541728 * lab.b;
    let s_ = lab.l - 0.0894841775 * lab.a - 1.2914855480 * lab.b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
}

fn linear_to_srgb(linear: [f32; 3]) -> [u8; 3] {
    [linear_channel_to_srgb_channel(linear[0]),
    linear_channel_to_srgb_channel(linear[1]),
    linear_channel_to_srgb_channel(linear[2])]
}

fn oklab_to_rgb(lab: &Lab) -> [u8; 3] {
    let linear = oklab_to_linear_srgb(lab);

    linear_to_srgb(linear)
}

fn linear_channel_to_srgb_channel(c: f32) -> u8 {
    let v = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };

    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}


pub fn to_hsl(color: &Lab) -> [f32; 3] {
    let rgb_normal = oklab_to_rgb(&color)
        .map(|color| color as f32 / 255.0);

    let mut iter = rgb_normal.iter();
    let first = *iter.next().unwrap();

    let (min, max) = iter.fold((first, first), |(min, max), &x| {
        (min.min(x), max.max(x))
    });

    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta == 0.0 {
        return [0.0, 0.0, l];
    }

    let s = delta / (1.0 - (2.0 * l - 1.0).abs());

    let mut hue = if rgb_normal[0] == max {
        (rgb_normal[1] - rgb_normal[2]) / (max - min)
    }else if rgb_normal[1] == max {
        2.0 + ((rgb_normal[2] - rgb_normal[0]) / (max - min))
    }else {
        4.0 + ((rgb_normal[0] - rgb_normal[1]) / (max - min))
    };

    if hue < 0.0 {
        hue += 6.0;
    }

    hue *= 60.0;

    [hue, s, l]
}