use serde::{Deserialize, Serialize};

pub const MAX_UNIT_CENTS: u64 = 100_000_000;
pub const MAX_QUANTITY: u32 = 9999;
pub const MAX_LINES: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
pub struct DraftLine {
    pub name: String,
    pub category: Category,
    pub quantity: u32,
    pub unit_cents: u64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Equipment,
    Service,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Quote {
    pub lines: Vec<PricedLine>,
    pub total_cents: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PricedLine {
    #[serde(flatten)]
    pub item: DraftLine,
    pub amount_cents: u64,
}

pub fn price(items: Vec<DraftLine>) -> Result<Quote, &'static str> {
    if items.len() > MAX_LINES {
        return Err("一份报价最多 100 行");
    }
    let mut total_cents = 0;
    let mut lines = Vec::with_capacity(items.len());
    for mut item in items {
        item.name = item.name.trim().to_owned();
        if item.name.is_empty() || item.name.chars().count() > 100 {
            return Err("品名须为 1–100 个字符");
        }
        if !(1..=MAX_QUANTITY).contains(&item.quantity) || item.unit_cents > MAX_UNIT_CENTS {
            return Err("数量或单价超出范围");
        }
        let amount_cents = u64::from(item.quantity) * item.unit_cents;
        total_cents += amount_cents;
        lines.push(PricedLine { item, amount_cents });
    }
    Ok(Quote { lines, total_cents })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(quantity: u32, unit_cents: u64) -> DraftLine {
        DraftLine {
            name: "示例设备".into(),
            category: Category::Equipment,
            quantity,
            unit_cents,
        }
    }

    #[test]
    fn exact_integer_totals_and_empty_drafts() {
        let quote = price(vec![item(3, 1999), item(2, 1)]).unwrap();
        assert_eq!(quote.lines[0].amount_cents, 5997);
        assert_eq!(quote.total_cents, 5999);
        assert_eq!(price(vec![]).unwrap().total_cents, 0);
    }

    #[test]
    fn rejects_invalid_inputs() {
        assert!(price(vec![item(0, 200)]).is_err());
        assert!(price(vec![item(1, MAX_UNIT_CENTS + 1)]).is_err());
        assert!(price(vec![item(1, 10); MAX_LINES + 1]).is_err());
    }
}
