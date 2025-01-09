use color_eyre::Result;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct MailerConfig {
    pub username: String,
    pub password: String,
    pub relay: String,
    pub port: u16,
    pub tls: bool,
}

#[derive(Clone)]
pub struct Mailer {
    address: String,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl Mailer {
    pub fn new(config: &MailerConfig) -> Result<Self> {
        let creds = Credentials::new(config.username.to_string(), config.password.to_string());
        let builder = if config.tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.relay)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.relay)?
        };
        let mailer = builder.port(config.port).credentials(creds).build();

        Ok(Self {
            address: config.username.to_string(),
            mailer,
        })
    }

    pub async fn send(&self, subject: &str, body: &str, to: &str) {
        let email = Message::builder()
            .from(self.address.parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .body(String::from(body))
            .unwrap();

        match self.mailer.send(email).await {
            Ok(_) => info!("Email sent successfully!"),
            Err(e) => error!("Could not send email: {:?}", e),
        }
    }
}

#[tokio::test]
async fn test_mailer() -> Result<()> {
    let mailer = Mailer::new(&MailerConfig {
        username: "username".to_string(),
        password: "password".to_string(),
        relay: "relay".to_string(),
        port: 587,
        tls: false,
    })?;
    mailer
        .send("Test", "This is a test email", "name@host")
        .await;

    Ok(())
}
