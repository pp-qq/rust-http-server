use hyper::service::{make_service_fn, service_fn};
use hyper::Method;
use hyper::StatusCode;
use hyper::{Body, Error, Request, Response, Server};
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::str;
use std::sync::Arc;
use std::{convert::Infallible, net::SocketAddr};
use tera::{Context, Tera};
use tokio::sync::Mutex;
use uuid::Uuid;

static TEMPLATE: &str = "Hello, {{name}}!";

async fn handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, world!!".into()))
}

async fn handle_with_body(req: Request<Body>, tera: Arc<Tera>) -> Result<Response<Body>, Error> {
    let body = hyper::body::to_bytes(req.into_body()).await?;
    let body = str::from_utf8(&body).unwrap();
    let name = body.strip_prefix("name=").unwrap();

    let mut ctx = Context::new();
    ctx.insert("name", name);
    let rendered = tera.render("hello", &ctx).unwrap();

    Ok(Response::new(rendered.into()))
}

struct Post {
    id: Uuid,
    title: String,
    content: String,
}

impl Post {
    // テンプレートを使って投稿を文字列にレンダリングする関数
    fn render(&self, tera: Arc<Tera>) -> String {
        ("aa");
    }
}

// リクエストパスの/posts/(post_id)からpost_idの部分を取り出す関数
fn get_id(req: &Request<Body>) -> Uuid {
    /*略*/
}

async fn find_post(
    req: Request<Body>,
    tera: Arc<Tera>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Response<Body>, Error> {
    let id = get_id(&req);

    let post = conn
        .lock()
        .await
        .query_row(
            "SELECT id, title, content FROM posts WHERE id = ?1",
            params![id],
            |row| {
                Ok(Post {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                })
            },
        )
        .optional()
        .unwrap();

    /*略*/
    match post {
        Some(post) => Ok(Response::new(post.render(tera).into())),
        None => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}

//Arc<Tera>を中継するroute関数
async fn route(
    req: Request<Body>,
    tera: Arc<Tera>,
    conn: Arc<Mutex<Connection>>,
) -> Result<Response<Body>, Error> {
    match (req.uri().path(), req.method().as_str()) {
        ("/", "GET") => handle_with_body(req, tera).await,
        ("/", _) => handle(req).await.map_err(|e| match e {}),
        ("/posts", "POST") => create_post(req, tera, conn).await,
        (path, "GET") if path.status_with("/posts/") => find_post(req, tera, conn).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
    //HTTPメソッドがPOSTのときにリクエストをhandle_with_body関数に振り分ける
    match *req.method() {
        Method::POST => handle_with_body(req, tera).await,
        _ => handle(req).await.map_err(|e| match e {}),
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let mut tera = Tera::default();
    tera.add_raw_template("hello", TEMPLATE).unwrap();
    let tera = Arc::new(tera);

    tera.add_raw_template(
        "post",
        "id: {{id}}\ntitle: {{title}}\ncontent:\n{{content}}",
    )
    .unwrap();

    let conn = Connection::open_in_memory().unwrap();
    let conn = Arc::new(Mutex::new(conn));

    let make_svc = make_service_fn(|_conn| {
        //クロージャからteraをclone
        //cloneはスレッド数分
        let tera = tera.clone();
        let conn = conn.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                //teraをasync task数分clone
                //async taskは、非同期ランタイムが実行をスケジュールする単位
                route(req, tera.clone(), conn.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

//derive(Deserialize)で、対応するデータフォーマットからtitleとcontentを取り出す
#[derive(Deserialize)]
struct NewPost<'a> {
    title: &'a str,
    content: &'a str,
}

async fn create_post(
    req: Request<Body>,
    _: Arc<Tera>,
    //MutexによるConnectionの排他制御
    conn: Arc<Mutex<Connection>>,
) -> Result<Response<Body>, Error> {
    let body = hyper::body::to_bytes(req.into_body()).await?;
    //serde_urlencoded::from_bytesでフォーラムデータを取り出す
    let new_post = serde_urlencoded::from_bytes::<NewPost>(&body).unwrap();
    //DBに登録するためのUUIDの生成
    let id = Uuid::new_v4();

    //.lockでロックを取得
    conn.lock()
        .await
        .execute(
            "INSERT INTO posts(id, title, content) VALUES (?1, ?2, ?3)",
            params![&id, new_post.title, new_post.content],
        )
        .unwrap();

    //IDをクライアントに返す
    Ok(Response::new(id.to_string().into()))
}
