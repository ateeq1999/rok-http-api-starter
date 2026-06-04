use lettre::message::{header::ContentType, Mailbox, SinglePart};
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::Tokio1Executor;
use lettre::{AsyncTransport, Message};

#[derive(Clone)]
pub struct Mailer {
    from: Mailbox,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl Mailer {
    pub fn new(host: &str, port: u16, from: &str) -> Result<Self, lettre::transport::smtp::Error> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
            .build();
        let from: Mailbox = from.parse().expect("invalid SMTP_FROM address");
        Ok(Self { from, transport })
    }

    fn render(template: &str, pairs: &[(&str, &str)]) -> String {
        let mut html = template.to_string();
        for (k, v) in pairs {
            html = html.replace(&format!("{{{{{}}}}}", k), v);
        }
        html
    }

    pub async fn send_otp(
        &self,
        to: &str,
        name: &str,
        code: &str,
        verify_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html = Self::render(
            include_str!("../templates/email_otp.html"),
            &[("name", name), ("code", code), ("verify_url", verify_url)],
        );
        self.send_raw(to, "Verify your email address", &html).await
    }

    pub async fn send_password_reset(
        &self,
        to: &str,
        name: &str,
        token: &str,
        reset_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let html = Self::render(
            include_str!("../templates/email_reset.html"),
            &[("name", name), ("token", token), ("reset_url", reset_url)],
        );
        self.send_raw(to, "Reset your password", &html).await
    }

    async fn send_raw(
        &self,
        to: &str,
        subject: &str,
        html: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let to: Mailbox = to.parse()?;
        let email = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject.to_string())
            .singlepart(SinglePart::builder().header(ContentType::TEXT_HTML).body(html.to_string()))?;
        self.transport.send(email).await?;
        Ok(())
    }
}
