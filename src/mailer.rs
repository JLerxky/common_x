use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tracing::{error, info};

#[derive(Clone)]
pub struct Mailer {
    address: String,
    mailer: SmtpTransport,
}

impl Mailer {
    pub fn new(username: &str, password: &str, relay: &str, port: u16) -> Self {
        let creds = Credentials::new(username.to_string(), password.to_string());
        let smtp = SmtpTransport::starttls_relay(relay)
            .unwrap()
            .port(port)
            .credentials(creds)
            .build();

        Self {
            address: username.to_string(),
            mailer: smtp,
        }
    }

    pub fn send(&self, subject: &str, body: &str, to: &str) {
        let email = Message::builder()
            .from(self.address.parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .body(String::from(body))
            .unwrap();

        match self.mailer.send(&email) {
            Ok(_) => info!("Email sent successfully!"),
            Err(e) => error!("Could not send email: {:?}", e),
        }
    }
}
