use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use crate::models::DbConfig;

pub type TiberiusClient = Client<tokio_util::compat::Compat<TcpStream>>;

pub async fn connect(db_config: &DbConfig) -> Result<TiberiusClient, String> {
  let mut config = Config::new();
  config.host(&db_config.host);
  config.port(db_config.port);
  config.database(&db_config.database);
  config.authentication(AuthMethod::sql_server(
    &db_config.username,
    &db_config.password,
  ));
  config.trust_cert();

  let tcp = TcpStream::connect(config.get_addr())
    .await
    .map_err(|e| format!("TCP connection failed: {e}"))?;

  tcp
    .set_nodelay(true)
    .map_err(|e| format!("Set nodelay failed: {e}"))?;

  let client = Client::connect(config, tcp.compat_write())
    .await
    .map_err(|e| format!("TDS connection failed: {e}"))?;

  Ok(client)
}
