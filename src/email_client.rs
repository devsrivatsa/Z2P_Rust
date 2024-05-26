use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

pub struct EmailClient {
    sender: SubscriberEmail,
    base_url: String,
    http_client: Client,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            sender,
            base_url,
            http_client,
            authorization_token,
        }
    }
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        //this needs to change
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        let builder = self
            .http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token", //this is as per postmark - a custom header
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }


}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_ok, assert_err};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    //use fake::faker::lorem::raw::Paragraph;
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    //implementing a matcher for matching body
    struct SendEmailBodyMatcher;
    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            //try to parse the body as a json value
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                //check if all mandatory fields are populated without inspecting the field values
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false //if parsing failed, do not match the request
            }
        }
    }
    //generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    //generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    //generate a random subscriber email
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    //get a test instance of EmailClient
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200)
        )
    }
    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = email_client(mock_server.uri());
        Mock::given(header_exists("X-Postmark-Server-Token")) //this is also as per postmark
            .and(header("Content-Type", "application/json")) //this might also be as per postmark
            .and(path("/email")) //this is also as per postmark
            .and(method("POST"))
            .and(SendEmailBodyMatcher) //using the custom implemented matcher
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        //act
        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        //assess
    }

    //new happy path
    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        //arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        //act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        //assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        //arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        //act
        let outcome = email_client.send_email(email(), &subject(), &content(), &content()).await;

        //assert
        assert_err!(outcome);
    }

    //test for timeout
    async fn send_email_times_out_if_the_server_takes_too_long() {
        //Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = email_client(mock_server.uri());
        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        //act
        let outcome = email_client.send_email(email(), &subject(), &content(), &content()).await;
        //assert
        assert_err!(outcome);
    }
}
