use axum::{
    Form,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use minijinja::context;
use serde::Deserialize;

use crate::{AppState, ForumError, ForumResult};

use super::forum_post::ForumPost;

pub async fn base_css(State(app_state): State<AppState>) -> ForumResult<Response> {
    static CSS: &str = grass::include!("assets/base.scss");
    let response = (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], CSS);

    Ok(response.into_response())
}

pub async fn show_posts(State(app_state): State<AppState>) -> ForumResult<Html<String>> {
    let posts = {
        let conn = app_state
            .database
            .lock()
            .map_err(|poison_err| ForumError::LockError(format!("{:?}", poison_err)))?;

        ForumPost::get_all(&*conn)?
    };

    let template = app_state
        .template
        .get_template("show_posts")
        .map_err(ForumError::TemplateError)?;

    let rendered = template
        .render(context! {
            posts => posts,
        })
        .map_err(ForumError::TemplateError)?;

    Ok(Html(rendered))
}

pub async fn show_create(State(app_state): State<AppState>) -> ForumResult<Html<String>> {
    let template = app_state
        .template
        .get_template("show_create")
        .map_err(ForumError::TemplateError)?;

    let rendered = template
        .render(context! {})
        .map_err(ForumError::TemplateError)?;

    Ok(Html(rendered))
}

#[derive(Deserialize)]
pub struct CreatePost {
    pub author: String,
    pub message: String,
}

pub async fn handle_create(
    State(app_state): State<AppState>,
    Form(payload): Form<CreatePost>,
) -> ForumResult<Response> {
    if payload.author.trim().is_empty() {
        return Err(ForumError::ValidationError("Author cannot be empty"));
    }

    if payload.message.trim().is_empty() {
        return Err(ForumError::ValidationError("Message cannot be empty"));
    }

    let created_post = {
        let conn = app_state
            .database
            .lock()
            .map_err(|poison_err| ForumError::LockError(format!("{:?}", poison_err)))?;

        ForumPost::post_save(&*conn, payload.author, payload.message)?
    };

    let response = Response::builder()
        .status(302)
        .header(header::LOCATION, format!("/post/{}", created_post.id))
        .body(Body::empty())
        .map_err(ForumError::HttpError)?;

    Ok(response)
}

pub async fn show_post(
    State(app_state): State<AppState>,
    Path(post_id): Path<usize>,
) -> ForumResult<Html<String>> {
    let found_post = {
        let conn = app_state
            .database
            .lock()
            .map_err(|poison_err| ForumError::LockError(format!("{:?}", poison_err)))?;

        ForumPost::get(&*conn, post_id)?
    };

    let template = app_state
        .template
        .get_template("show_post")
        .map_err(ForumError::TemplateError)?;

    let rendered = template
        .render(context! {
            post => found_post,
        })
        .map_err(ForumError::TemplateError)?;

    Ok(Html(rendered))
}
