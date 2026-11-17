// Required Notice: Copyright 2026 Alex Yip. SmarFit（智装工场）. Source marker: ximo. https://github.com/Yipalex/SmarFit

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use smarfit_core::wifi::Plan;
use smarfit_core::{DraftLine, Quote, price};
use worker::wasm_bindgen::JsValue;
use worker::*;

mod questionnaire;
use questionnaire::{Inquiry, valid_key};

#[derive(Default, Serialize, Deserialize)]
struct Workspace {
    projects: Vec<Project>,
    catalog: Vec<DraftLine>,
}

#[derive(Serialize, Deserialize)]
struct Project {
    name: String,
    requirements: String,
    draft: Quote,
    published: Vec<Quote>,
}

#[derive(Deserialize)]
struct Change {
    revision: u32,
    #[serde(flatten)]
    action: Action,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Action {
    CreateProject {
        name: String,
        requirements: String,
    },
    SetQuestionnaire {
        key: Option<String>,
    },
    ResolveInquiry {
        id: String,
        create_project: bool,
    },
    SaveProduct {
        index: Option<usize>,
        item: DraftLine,
    },
    SaveWifi {
        project: usize,
        plan: Plan,
    },
    SaveQuote {
        project: usize,
        lines: Vec<DraftLine>,
        publish: bool,
    },
}

#[derive(Deserialize)]
struct Stored {
    revision: u32,
    document: String,
    questionnaire_key: Option<String>,
    inbox: String,
    wifi: String,
}

impl Workspace {
    fn apply(&mut self, action: Action) -> std::result::Result<(), &'static str> {
        match action {
            Action::CreateProject { name, requirements } => {
                let name = name.trim();
                if name.is_empty()
                    || name.chars().count() > 80
                    || requirements.chars().count() > 500
                    || self.projects.len() >= 10
                {
                    return Err(
                        "项目名称须为 1–80 字，需求最多 500 字，每个试验工作台最多 10 个项目",
                    );
                }
                self.projects.push(Project {
                    name: name.into(),
                    requirements: requirements.trim().into(),
                    draft: price(vec![])?,
                    published: vec![],
                });
            }
            Action::SetQuestionnaire { .. }
            | Action::ResolveInquiry { .. }
            | Action::SaveWifi { .. } => unreachable!(),
            Action::SaveProduct { index, mut item } => {
                item.quantity = 1;
                let mut checked = price(vec![item])?;
                let item = checked.lines.remove(0).item;
                if let Some(index) = index {
                    *self.catalog.get_mut(index).ok_or("产品不存在")? = item;
                } else {
                    if self.catalog.len() >= 50 {
                        return Err("试验产品库最多 50 项");
                    }
                    self.catalog.push(item);
                }
            }
            Action::SaveQuote {
                project,
                lines,
                publish,
            } => {
                let project = self.projects.get_mut(project).ok_or("项目不存在")?;
                let quote = price(lines)?;
                if publish {
                    if quote.lines.is_empty() || project.published.len() >= 10 {
                        return Err("发布须包含至少一项，每个项目最多发布 10 版");
                    }
                    project.published.push(quote.clone());
                }
                project.draft = quote;
            }
        }
        Ok(())
    }
}

#[event(fetch)]
async fn main(mut req: Request, env: Env, _: Context) -> Result<Response> {
    let path = req.path();
    let result = match (req.method(), path.as_str()) {
        (Method::Get, "/api/health") => Response::from_json(&json!({ "status": "ok" })),
        (Method::Get, "/api/version") => Response::from_json(&json!({
            "name": "SmarFit",
            "marker": "ximo",
            "version": env!("CARGO_PKG_VERSION"),
            "commit": env!("SMARFIT_BUILD_COMMIT")
        })),
        (Method::Post, "/api/price") => match read_json(&mut req).await {
            Ok(lines) => match price(lines) {
                Ok(quote) => Response::from_json(&quote),
                Err(message) => Response::error(message, 422),
            },
            Err(response) => response,
        },
        (Method::Post, "/api/wifi/coverage") => match read_json::<Plan>(&mut req).await {
            Ok(plan) => match plan.coverage() {
                Ok(coverage) => Response::from_json(&coverage),
                Err(message) => Response::error(message, 422),
            },
            Err(response) => response,
        },
        (Method::Get | Method::Post, "/api/workspace") => workspace(req, env).await,
        (_, path) if path.starts_with("/api/questionnaires/") => {
            questionnaire::handle(req, env.d1("DB")?, &path["/api/questionnaires/".len()..]).await
        }
        _ => Response::error("接口不存在", 404),
    };
    match result {
        Ok(mut response) => {
            response.headers_mut().set("Cache-Control", "no-store")?;
            Ok(response)
        }
        Err(error) => {
            console_error!("SmarFit API: {error}");
            Response::error("暂时无法处理请求，请稍后重试", 500)
        }
    }
}

async fn read_json<T: serde::de::DeserializeOwned>(
    req: &mut Request,
) -> std::result::Result<T, Result<Response>> {
    let bytes = req
        .bytes()
        .await
        .map_err(|_| Response::error("无法读取请求", 400))?;
    if bytes.len() > 65_536 {
        return Err(Response::error("请求过大", 413));
    }
    serde_json::from_slice(&bytes).map_err(|_| Response::error("请求格式无效", 400))
}

async fn workspace(mut req: Request, env: Env) -> Result<Response> {
    let authorization = req.headers().get("Authorization")?.unwrap_or_default();
    let Some(token) = authorization
        .strip_prefix("Bearer ")
        .filter(|token| valid_key(token))
    else {
        return Response::error("缺少试验工作台凭证", 401);
    };
    let db = env.d1("DB")?;
    let row = db
        .prepare(
            "SELECT revision, document, questionnaire_key, inbox, wifi FROM workspaces WHERE token = ?1",
        )
        .bind(&[token.into()])?
        .first::<Stored>(None)
        .await?;
    let (revision, mut workspace, mut questionnaire_key, mut inbox, mut wifi) = match row {
        Some(row) => (
            row.revision,
            serde_json::from_str(&row.document)?,
            row.questionnaire_key,
            serde_json::from_str::<Vec<Inquiry>>(&row.inbox)?,
            serde_json::from_str::<BTreeMap<usize, Plan>>(&row.wifi)?,
        ),
        None => (0, Workspace::default(), None, vec![], BTreeMap::new()),
    };
    if req.method() == Method::Get {
        return Response::from_json(
            &json!({ "revision": revision, "workspace": workspace, "questionnaire_key": questionnaire_key, "inbox": inbox, "wifi": wifi }),
        );
    }
    let change: Change = match read_json(&mut req).await {
        Ok(change) => change,
        Err(response) => return response,
    };
    if change.revision != revision {
        return Response::error("工作台版本已更新，请刷新后重试", 409);
    }
    match change.action {
        Action::SaveWifi { project, plan } => {
            if workspace.projects.get(project).is_none() {
                return Response::error("项目不存在", 404);
            }
            if let Err(message) = plan.validate() {
                return Response::error(message, 422);
            }
            wifi.insert(project, plan);
        }
        Action::SetQuestionnaire { key } => {
            if key
                .as_deref()
                .is_some_and(|key| !valid_key(key) || key == token)
            {
                return Response::error("问卷链接无效", 422);
            }
            questionnaire_key = key;
        }
        Action::ResolveInquiry { id, create_project } => {
            let Some(index) = inbox.iter().position(|entry| entry.id == id) else {
                return Response::error("需求不存在或已处理", 404);
            };
            let inquiry = &inbox[index];
            if create_project
                && let Err(message) = workspace.apply(Action::CreateProject {
                    name: inquiry.name.clone(),
                    requirements: inquiry.requirements(),
                })
            {
                return Response::error(message, 422);
            }
            inbox.remove(index);
        }
        action => {
            if let Err(message) = workspace.apply(action) {
                return Response::error(message, 422);
            }
        }
    }
    let document = serde_json::to_string(&workspace)?;
    if document.len() > 500_000 {
        return Response::error("试验工作台已达到存储上限", 422);
    }
    let updated = db.prepare(
        "INSERT INTO workspaces (token, revision, document, questionnaire_key, inbox, wifi) VALUES (?1, ?2 + 1, ?3, ?4, ?5, ?6)
         ON CONFLICT (token) DO UPDATE SET revision = excluded.revision, document = excluded.document,
             questionnaire_key = excluded.questionnaire_key, inbox = excluded.inbox, wifi = excluded.wifi
         WHERE workspaces.revision = ?2"
    ).bind(&[token.into(), revision.into(), document.into(), questionnaire_key.as_deref().map(Into::into).unwrap_or(JsValue::NULL), serde_json::to_string(&inbox)?.into(), serde_json::to_string(&wifi)?.into()])?.run().await?;
    if updated.meta()?.and_then(|meta| meta.changes) != Some(1) {
        return Response::error("工作台版本已更新，请刷新后重试", 409);
    }
    Response::from_json(
        &json!({ "revision": revision + 1, "workspace": workspace, "questionnaire_key": questionnaire_key, "inbox": inbox, "wifi": wifi }),
    )
}
