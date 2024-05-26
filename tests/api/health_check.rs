use crate::helpers::spawn_app;
#[tokio::test]
async fn health_check_works() {
    //spawn app and get address
    let app = spawn_app().await; //this will spawn the server, and return the bound address and port

    //send a request via request client to check the health of the server
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", &app.address)) //constructs a RequestBuilder object with the specified url
        .send() //sends the request and returns a future
        .await //polls for response
        .expect("Failed to execute request!");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

