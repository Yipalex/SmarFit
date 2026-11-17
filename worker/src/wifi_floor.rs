use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use smarfit_core::wifi::{Plan, Point};
use worker::*;

const MAX_IMAGE: usize = 180_000;

#[derive(Serialize, Deserialize)]
pub struct Floor {
    jpeg: String,
    pixel_width: u16,
    pixel_height: u16,
    from: Point,
    to: Point,
    distance_cm: u16,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FloorUpdate {
    Set { floor: Floor },
    Remove,
}

impl Floor {
    fn scale(&self) -> Option<(u16, u16)> {
        if !(1..=1600).contains(&self.pixel_width)
            || !(1..=1600).contains(&self.pixel_height)
            || [self.from, self.to]
                .iter()
                .any(|p| p.x > 1000 || p.y > 1000)
            || self.distance_cm < 10
            || self.distance_cm > 7071
        {
            return None;
        }
        let dx = f64::from(self.to.x) - f64::from(self.from.x);
        let dy = f64::from(self.to.y) - f64::from(self.from.y);
        let width = f64::from(self.pixel_width);
        let height = f64::from(self.pixel_height);
        let segment = (dx * width).hypot(dy * height) / 1000.0;
        if segment < 5.0 {
            return None;
        }
        let cm_per_pixel = f64::from(self.distance_cm) / segment;
        Some((
            (width * cm_per_pixel).round() as u16,
            (height * cm_per_pixel).round() as u16,
        ))
    }

    pub fn validate(&self, plan: &Plan) -> bool {
        if self.jpeg.len() > MAX_IMAGE {
            return false;
        }
        let Ok(bytes) = STANDARD.decode(&self.jpeg) else {
            return false;
        };
        if jpeg_dimensions(&bytes) != Some((self.pixel_width, self.pixel_height)) {
            return false;
        }
        self.scale().is_some_and(|(width, height)| {
            width.abs_diff(plan.width_cm) <= 1 && height.abs_diff(plan.height_cm) <= 1
        })
    }
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u16, u16)> {
    if !bytes.starts_with(&[0xff, 0xd8]) || !bytes.ends_with(&[0xff, 0xd9]) {
        return None;
    }
    let mut offset = 2;
    while bytes.get(offset) == Some(&0xff) {
        let marker = *bytes.get(offset + 1)?;
        let length =
            u16::from_be_bytes(bytes.get(offset + 2..offset + 4)?.try_into().ok()?) as usize;
        if length < 2 || offset + 2 + length > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc2) && length >= 8 {
            let height = u16::from_be_bytes(bytes.get(offset + 5..offset + 7)?.try_into().ok()?);
            let width = u16::from_be_bytes(bytes.get(offset + 7..offset + 9)?.try_into().ok()?);
            return Some((width, height));
        }
        if marker == 0xda {
            return None;
        }
        offset += 2 + length;
    }
    None
}

pub async fn load(db: &D1Database, token: &str, project: usize) -> Result<Response> {
    let value = db
        .prepare("SELECT floor FROM wifi_floors WHERE token = ?1 AND project = ?2")
        .bind(&[token.into(), (project as u32).into()])?
        .first::<String>(Some("floor"))
        .await?;
    let floor = value
        .map(|json| serde_json::from_str::<Floor>(&json))
        .transpose()?;
    Response::from_json(&floor)
}

pub fn update(
    db: &D1Database,
    token: &str,
    project: usize,
    floor: FloorUpdate,
) -> Result<D1PreparedStatement> {
    let project = project as u32;
    // D1 batches run in one transaction. The preceding workspace CAS must have written one row.
    match floor {
        FloorUpdate::Set { floor } => db
            .prepare(
                "INSERT INTO wifi_floors (token, project, floor)
             SELECT ?1, ?2, ?3 WHERE changes() = 1
             ON CONFLICT (token, project) DO UPDATE SET floor = excluded.floor",
            )
            .bind(&[
                token.into(),
                project.into(),
                serde_json::to_string(&floor)?.into(),
            ]),
        FloorUpdate::Remove => db
            .prepare("DELETE FROM wifi_floors WHERE token = ?1 AND project = ?2 AND changes() = 1")
            .bind(&[token.into(), project.into()]),
    }
}
