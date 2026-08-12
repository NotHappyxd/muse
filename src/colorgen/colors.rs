use crate::colorgen::conversions;

#[derive(Clone, Copy, Debug, Default)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
}

impl Lab {
    #[inline]
    pub fn distance(&self, other: &Self) -> f32 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;

        dl * dl + da * da + db * db
    }

    #[warn(dead_code)]
    pub fn accent_distance_squared(&self, other: &Self) -> f32 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;

        (dl * dl * 0.8) + (da * da * 1.2) + (db * db * 1.2)
    }

    pub fn chroma(&self) -> f32 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    pub fn scale_chroma(&mut self, chroma: f32) {
        self.a *= chroma;
        self.b *= chroma;
    }

    pub fn with_chroma(&mut self, chroma: f32) {
        let current = self.chroma();

        if current <= f32::EPSILON {
            return;
        }

        let scale = chroma / current;
        self.scale_chroma(scale);
    }
}

impl Oklch {
    pub fn from_oklab(lab: &Lab) -> Self {
        let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
        let h = lab.b.atan2(lab.a);
        Oklch { l: lab.l, c, h }
    }

    pub fn to_oklab(&self) -> Lab {
        Lab {
            l: self.l,
            a: self.c * self.h.cos(),
            b: self.c * self.h.sin(),
        }
    }
}

pub fn find_max_chroma_oklab(mut lch: Oklch) -> Lab {
    let mut low_c = 0.0f32;
    let mut high_c = lch.c;
    let mut best_lab = lch.to_oklab();

    for _ in 0..8 {
        let mid_c = (low_c + high_c) / 2.0;
        lch.c = mid_c;
        let lab = lch.to_oklab();
        let linear = conversions::oklab_to_linear_srgb(&lab);

        if linear[0] >= 0.0
            && linear[0] <= 1.0
            && linear[1] >= 0.0
            && linear[1] <= 1.0
            && linear[2] >= 0.0
            && linear[2] <= 1.0
        {
            best_lab = lab;
            low_c = mid_c; // Push chroma higher!
        } else {
            high_c = mid_c; // Out of gamut, scale back chroma
        }
    }

    best_lab
}

pub fn luminance(rgb: [u8; 3]) -> f32 {
    let [r, g, b] = rgb.map(|color| color as f32 / 255.0).map(|normal| {
        if normal <= 0.03928 {
            normal / 12.92
        } else {
            ((normal + 0.055) / 1.055).powf(2.4)
        }
    });

    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub fn wcag_contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    let la = luminance(a);
    let lb = luminance(b);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn true_gamut_max_chroma(l: f32, h: f32) -> f32 {
    let lab = find_max_chroma_oklab(Oklch { l, c: 0.4, h });
    Oklch::from_oklab(&lab).c
}
