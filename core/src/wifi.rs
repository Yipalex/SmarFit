use serde::{Deserialize, Serialize};

const GRID: usize = 24;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    Partition,
    Brick,
    Concrete,
}

impl Material {
    fn loss_db(self) -> f64 {
        match self {
            Self::Partition => 4.0,
            Self::Brick => 8.0,
            Self::Concrete => 16.0,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Wall {
    pub from: Point,
    pub to: Point,
    pub material: Material,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Plan {
    pub width_cm: u16,
    pub height_cm: u16,
    pub access_points: Vec<Point>,
    pub walls: Vec<Wall>,
}

#[derive(Serialize)]
pub struct Coverage {
    pub size: usize,
    pub levels: Vec<u8>,
    pub target_percent: u32,
}

impl Plan {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(100..=5000).contains(&self.width_cm) || !(100..=5000).contains(&self.height_cm) {
            return Err("户型宽度和深度须介于 1–50 米");
        }
        if self.access_points.len() > 8 || self.walls.len() > 24 {
            return Err("最多支持 8 个 AP 和 24 段墙");
        }
        let inside = |p: Point| p.x <= 1000 && p.y <= 1000;
        if !self.access_points.iter().copied().all(inside)
            || self
                .walls
                .iter()
                .any(|w| !inside(w.from) || !inside(w.to) || w.from == w.to)
        {
            return Err("墙体与 AP 点位必须在户型内，墙体需要两个不同端点");
        }
        Ok(())
    }

    pub fn coverage(&self) -> Result<Coverage, &'static str> {
        self.validate()?;
        let mut levels = Vec::with_capacity(GRID * GRID);
        let mut target_count = 0;
        for y in 0..GRID {
            for x in 0..GRID {
                let point = Point {
                    x: ((2 * x + 1) * 500 / GRID) as u16,
                    y: ((2 * y + 1) * 500 / GRID) as u16,
                };
                let signal = self
                    .access_points
                    .iter()
                    .map(|&ap| {
                        let dx = (f64::from(ap.x) - f64::from(point.x)) * f64::from(self.width_cm)
                            / 100_000.0;
                        let dy = (f64::from(ap.y) - f64::from(point.y)) * f64::from(self.height_cm)
                            / 100_000.0;
                        let distance = dx.hypot(dy).max(1.0);
                        let attenuation: f64 = self
                            .walls
                            .iter()
                            .filter(|wall| crosses(ap, point, wall))
                            .map(|wall| wall.material.loss_db())
                            .sum();
                        // Trial model: generic 18 dBm EIRP, 47 dB reference loss at 1 m,
                        // indoor path-loss exponent 3 and illustrative wall loss.
                        18.0 - 47.0 - 30.0 * distance.log10() - attenuation
                    })
                    .fold(f64::NEG_INFINITY, f64::max);
                let level = if signal >= -67.0 {
                    target_count += 1;
                    3
                } else if signal >= -75.0 {
                    2
                } else if signal.is_finite() {
                    1
                } else {
                    0
                };
                levels.push(level);
            }
        }
        Ok(Coverage {
            size: GRID,
            levels,
            target_percent: (100 * target_count + (GRID * GRID / 2) as u32) / (GRID * GRID) as u32,
        })
    }
}

fn crosses(ap: Point, sample: Point, wall: &Wall) -> bool {
    fn orientation(a: Point, b: Point, c: Point) -> i32 {
        let delta = |lhs: u16, rhs: u16| i32::from(lhs) - i32::from(rhs);
        delta(b.x, a.x) * delta(c.y, a.y) - delta(b.y, a.y) * delta(c.x, a.x)
    }
    let a = orientation(ap, sample, wall.from);
    let b = orientation(ap, sample, wall.to);
    let c = orientation(wall.from, wall.to, ap);
    let d = orientation(wall.from, wall.to, sample);
    // A touch at an endpoint or a ray lying along the wall contributes no loss.
    a * i32::signum(b) < 0 && c * i32::signum(d) < 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concrete_reduces_coverage_only_behind_wall() {
        let mut plan = Plan {
            width_cm: 2000,
            height_cm: 1000,
            access_points: vec![Point { x: 150, y: 500 }],
            walls: vec![],
        };
        let clear = plan.coverage().unwrap();
        plan.walls.push(Wall {
            from: Point { x: 500, y: 0 },
            to: Point { x: 500, y: 1000 },
            material: Material::Concrete,
        });
        let obstructed = plan.coverage().unwrap();
        assert_eq!(
            clear.levels[GRID * 12 + 3],
            obstructed.levels[GRID * 12 + 3]
        );
        assert!(clear.levels[GRID * 12 + 16] > obstructed.levels[GRID * 12 + 16]);
        assert!(clear.target_percent > obstructed.target_percent);
        plan.access_points.clear();
        assert!(
            plan.coverage()
                .unwrap()
                .levels
                .iter()
                .all(|&level| level == 0)
        );
    }

    #[test]
    fn rejects_out_of_bounds_and_zero_length_walls() {
        let mut plan = Plan {
            width_cm: 1200,
            height_cm: 800,
            access_points: vec![Point { x: 1001, y: 500 }],
            walls: vec![],
        };
        assert!(plan.validate().is_err());
        plan.access_points.clear();
        let same = Point { x: 10, y: 10 };
        plan.walls.push(Wall {
            from: same,
            to: same,
            material: Material::Brick,
        });
        assert!(plan.validate().is_err());
    }
}
