use crate::{AppState, ForumError, ForumResult};
use axum::{
    Form,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use minijinja::context;
use serde::Deserialize;

use super::forum_post::{ForumPost, PostTreeNode};

pub async fn base_css(State(_app_state): State<AppState>) -> ForumResult<Response> {
    static CSS: &str = grass::include!("assets/base.scss");
    let response = (StatusCode::OK, [(header::CONTENT_TYPE, "text/css")], CSS);

    Ok(response.into_response())
}

pub async fn show_posts(State(app_state): State<AppState>) -> ForumResult<Html<String>> {
    let posts = { ForumPost::get_ops(&*get_connection(&app_state)?)? };

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

pub async fn show_create_post(State(app_state): State<AppState>) -> ForumResult<Html<String>> {
    let template = app_state
        .template
        .get_template("show_create")
        .map_err(ForumError::TemplateError)?;

    let rendered = template
        .render(context! {})
        .map_err(ForumError::TemplateError)?;

    Ok(Html(rendered))
}

pub async fn show_create_reply(
    State(app_state): State<AppState>,
    Path(post_id): Path<usize>,
) -> ForumResult<Html<String>> {
    let template = app_state
        .template
        .get_template("show_reply_create")
        .map_err(ForumError::TemplateError)?;

    let rendered = template
        .render(context! { post_id => post_id })
        .map_err(ForumError::TemplateError)?;

    Ok(Html(rendered))
}

#[derive(Deserialize)]
pub struct CreatePost {
    pub author: String,
    pub message: String,
}

pub async fn handle_create_post(
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
        ForumPost::post_save(
            &*get_connection(&app_state)?,
            payload.author,
            payload.message,
        )?
    };

    let response = Response::builder()
        .status(302)
        .header(header::LOCATION, format!("/post/{}", created_post.id))
        .body(Body::empty())
        .map_err(ForumError::HttpError)?;

    Ok(response)
}

#[derive(Deserialize)]
pub struct CreateReply {
    pub author: String,
    pub message: String,
}

pub async fn handle_delete_post(
    Path(post_id): Path<usize>,
    State(app_state): State<AppState>,
) -> ForumResult<Response> {
    let conn = get_connection(&app_state)?;

    ForumPost::soft_delete_post(&conn, post_id)?;

    let response = Response::builder()
        .status(302)
        .header(header::LOCATION, format!("/post/{}", post_id))
        .body(Body::empty())
        .map_err(ForumError::HttpError)?;

    Ok(response)
}

pub async fn handle_create_reply(
    Path(parent_id): Path<usize>, // Extract from URL instead of form
    State(app_state): State<AppState>,
    Form(payload): Form<CreateReply>,
) -> ForumResult<Response> {
    if payload.author.trim().is_empty() {
        return Err(ForumError::ValidationError("Author cannot be empty"));
    }

    if payload.message.trim().is_empty() {
        return Err(ForumError::ValidationError("Message cannot be empty"));
    }

    let created_post = {
        let conn = get_connection(&app_state)?;

        let parent = ForumPost::get(&*conn, parent_id).unwrap();

        ForumPost::reply_save(&parent, &*conn, payload.author, payload.message)?
    };

    // Redirect to the thread just replied
    let response = Response::builder()
        .status(302)
        .header(
            header::LOCATION,
            format!("/post/{}", created_post.root_id.unwrap()),
        )
        .body(Body::empty())
        .map_err(ForumError::HttpError)?;

    Ok(response)
}

pub async fn show_post(
    State(app_state): State<AppState>,
    Path(post_id): Path<usize>,
) -> ForumResult<Html<String>> {
    let conn = get_connection(&app_state)?;

    let found_post = ForumPost::get(&*conn, post_id)?;
    let reply_tree = PostTreeNode::build_tree(&*conn, post_id)?;

    let template = app_state
        .template
        .get_template("show_post")
        .map_err(ForumError::TemplateError)?;

    let rendered = template
        .render(context! {
            post => found_post,
            replies => reply_tree,
        })
        .map_err(ForumError::TemplateError)?;

    Ok(Html(rendered))
}

fn get_connection(
    app_state: &AppState,
) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, ForumError> {
    let conn = app_state
        .database
        .lock()
        .map_err(|poison_err| ForumError::LockError(format!("{:?}", poison_err)))?;
    Ok(conn)
}
