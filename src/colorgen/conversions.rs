use std::sync::OnceLock;
use crate::colorgen::kmeans::Lab;

static LINEAR_CACHE: OnceLock<[f32; 256]> = OnceLock::new();

fn get_linear_cache() -> &'static [f32; 256] {
    LINEAR_CACHE.get_or_init(|| {
        let mut cache = [0.0f32; 256];
        for i in 0..256 {
            let v = i as f32 / 255.0;
            cache[i] = if v > 0.04045 {
                ((v + 0.055) / 1.055).powf(2.4)
            } else {
                v / 12.92
            };
        }
        cache
    })
}

pub fn rgb_to_lab(rgb: [u8; 3]) -> Lab {
    let arr = rgb_to_lab_arr(rgb);
    
    Lab {
        l: arr[0],
        a: arr[1],
        b: arr[2],
    }
}

pub fn rgb_to_lab_arr(rgb: [u8; 3]) -> [f32; 3] {
    let linear_r = get_linear_cache()[rgb[0] as usize];
    let linear_g = get_linear_cache()[rgb[1] as usize];
    let linear_b = get_linear_cache()[rgb[2] as usize];

    let x = linear_r * 0.4124564 + linear_g * 0.3575761 + linear_b * 0.1804375;
    let y = linear_r * 0.2126729 + linear_g * 0.7151522 + linear_b * 0.0721750;
    let z = linear_r * 0.0193339 + linear_g * 0.1191920 + linear_b * 0.9503041;

    xyz_to_lab(x, y, z)
}

fn xyz_to_lab(x: f32, y: f32, z: f32) -> [f32; 3] {
    // D65 reference white
    const XN: f32 = 0.95047;
    const YN: f32 = 1.00000;
    const ZN: f32 = 1.08883;

    let fx = lab_f(x / XN);
    let fy = lab_f(y / YN);
    let fz = lab_f(z / ZN);

    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}


fn lab_f(t: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24389.0; // (6/29)^3
    const KAPPA: f32 = 24389.0 / 27.0;    // (29/3)^3

    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

// -----
pub fn lab_to_rgb(lab: &Lab) -> [u8; 3] {
    let (x, y, z) = lab_to_xyz([lab.l, lab.a, lab.b]);

    let r = x *  3.2404542 + y * -1.5371385 + z * -0.4985314;
    let g = x * -0.9692660 + y *  1.8760108 + z *  0.0415560;
    let b = x *  0.0556434 + y * -0.2040259 + z *  1.0572252;

    [
        linear_to_srgb(r),
        linear_to_srgb(g),
        linear_to_srgb(b),
    ]
}

fn lab_to_xyz(lab: [f32; 3]) -> (f32, f32, f32) {
    const XN: f32 = 0.95047;
    const YN: f32 = 1.00000;
    const ZN: f32 = 1.08883;

    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;

    (
        XN * lab_f_inv(fx),
        YN * lab_f_inv(fy),
        ZN * lab_f_inv(fz),
    )
}

fn lab_f_inv(t: f32) -> f32 {
    const DELTA: f32 = 6.0 / 29.0;

    if t > DELTA {
        t * t * t
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

fn linear_to_srgb(v: f32) -> u8 {
    let v = if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };

    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn to_hsl(color: &Lab) -> [f32; 3] {
    let rgb_normal = lab_to_rgb(&color)
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

pub fn hsl_to_rgb(hsl: [f32; 3]) -> [u8; 3] {
    let [h, s, l] = hsl;

    if s == 0.0 {
        let v = (l.clamp(0.0, 1.0) * 255.0).round() as u8;
        return [v, v, v];
    }

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r1 + m).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((g1 + m).clamp(0.0, 1.0) * 255.0).round() as u8,
        ((b1 + m).clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
