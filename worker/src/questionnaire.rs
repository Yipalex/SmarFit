use serde::{Deserialize, Serialize};
use serde_json::json;
use worker::*;

#[derive(Deserialize, Serialize)]
pub struct Inquiry {
    pub id: String,
    pub name: String,
    pub home: String,
    pub systems: Vec<String>,
    pub budget: String,
    pub notes: String,
}

#[derive(Deserialize)]
struct InboxRow {
    revision: u32,
    inbox: String,
}

pub fn valid_key(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl Inquiry {
    fn validate(&mut self) -> bool {
        self.name = self.name.trim().into();
        self.notes = self.notes.trim().into();
        self.systems.sort_unstable();
        self.systems.dedup();
        self.id.len() == 36
            && self.id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
            && (1..=80).contains(&self.name.chars().count())
            && self.notes.chars().count() <= 250
            && matches!(
                self.home.as_str(),
                "新房待装修" | "装修施工中" | "已入住改造"
            )
            && (1..=6).contains(&self.systems.len())
            && self.systems.iter().all(|name| {
                matches!(
                    name.as_str(),
                    "灯光" | "窗帘" | "网络" | "安防" | "影音" | "暖通"
                )
            })
            && matches!(
                self.budget.as_str(),
                "待确认" | "1 万元以下" | "1–3 万元" | "3–10 万元" | "10 万元以上"
            )
    }

    pub fn requirements(&self) -> String {
        let summary = format!(
            "房屋阶段：{}\n意向系统：{}\n预算：{}",
            self.home,
            self.systems.join("、"),
            self.budget,
        );
        if self.notes.is_empty() {
            summary
        } else {
            format!("{summary}\n补充需求：{}", self.notes)
        }
    }
}

pub async fn handle(mut req: Request, db: D1Database, key: &str) -> Result<Response> {
    if !valid_key(key) {
        return Response::error("此问卷链接无效或已关闭", 404);
    }
    let mut submission: Inquiry = match req.method() {
        Method::Get => {
            let enabled = db
                .prepare("SELECT 1 AS enabled FROM workspaces WHERE questionnaire_key = ?1")
                .bind(&[key.into()])?
                .first::<u32>(Some("enabled"))
                .await?;
            return if enabled.is_some() {
                Response::from_json(&json!({ "open": true }))
            } else {
                Response::error("此问卷链接无效或已关闭", 404)
            };
        }
        Method::Post => match super::read_json(&mut req).await {
            Ok(submission) => submission,
            Err(response) => return response,
        },
        _ => return Response::error("请求方式无效", 405),
    };
    if !submission.validate() {
        return Response::error("请检查项目称呼、房屋阶段、意向系统与预算", 422);
    }
    let id = submission.id.clone();
    for _ in 0..3 {
        let row = db
            .prepare("SELECT revision, inbox FROM workspaces WHERE questionnaire_key = ?1")
            .bind(&[key.into()])?
            .first::<InboxRow>(None)
            .await?;
        let Some(row) = row else {
            return Response::error("此问卷链接无效或已关闭", 404);
        };
        let mut inbox: Vec<Inquiry> = serde_json::from_str(&row.inbox)?;
        if inbox.iter().any(|entry| entry.id == id) {
            return Response::from_json(&json!({ "received": true }));
        }
        if inbox.len() >= 30 {
            return Response::error("需求收件箱已满，请联系链接提供者", 429);
        }
        inbox.push(submission);
        let updated = db
            .prepare(
                "UPDATE workspaces SET inbox = ?1, revision = revision + 1
             WHERE questionnaire_key = ?2 AND revision = ?3",
            )
            .bind(&[
                serde_json::to_string(&inbox)?.into(),
                key.into(),
                row.revision.into(),
            ])?
            .run()
            .await?;
        if updated.meta()?.and_then(|meta| meta.changes) == Some(1) {
            return Response::from_json(&json!({ "received": true }));
        }
        submission = inbox
            .pop()
            .ok_or_else(|| Error::RustError("missing submission".into()))?;
    }
    Response::error("同时提交的需求较多，请稍后重试", 409)
}
