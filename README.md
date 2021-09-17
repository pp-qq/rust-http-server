# rust-http-server
Aiming at http web server with SQLite implemented in Rust.
Implementing some functions.

```
$ cargo run &
$ curl -dtitle=test -d'content=test input' http://localhost:3000/posts
f443b0d9-da15-48c7-ae09-6d29bf5a43aa
$ curl http://localhost:3000/posts/f443b0d9-da15-48c7-ae09-6d29bf5a43aa
id: f443b0d9-da15-48c7-ae09-6d29bf5a43aa
titel: test
content: 
test input
```
