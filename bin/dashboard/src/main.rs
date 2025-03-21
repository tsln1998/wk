use crate::args::Args;
use anyhow::{Ok, Result};
use axum::serve;
use clap::Parser;
use database::migrations::Migrator;
use database::migrations::MigratorTrait;
use sea_orm::ConnectOptions;
use sea_orm::Database;
use sea_orm::DatabaseConnection;
use state::AppState;
use tokio::net::TcpListener;
use tokio::select;
use tokio::signal;
use tokio::sync::broadcast;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod api;
mod args;
mod prelude;
mod route;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    // configure logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    // parse command line arguments
    let args = Args::parse();

    // create a TCP listener and a database connection
    let listener = make_listener(&args).await?;
    let database = make_database(&args).await?;

    // create app state
    let state = AppState::new(args, database);

    // create a router
    let router = crate::route::make()
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http());

    // create shutdown signal receiver
    let mut shutdown = make_shutdown_signal();

    // start server
    let server = serve(listener, router).with_graceful_shutdown({
        async move {
            // TODO: spawn daemon task

            // wait for shutdown signal
            shutdown.recv().await.unwrap()
        }
    });

    tracing::info!("listening on {}", server.local_addr()?);

    // wait server stop
    server.await?;

    // TODO: wait daemon task stop

    // wait state persisted
    state.close().await?;

    Ok(())
}

/// Create a TCP listener bound to the address given in Args.
///
/// Takes Args as input and attempts to bind a TCP listener to the address given in
/// Args.listen. The listener is then returned.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot be bound to the given address.
async fn make_listener(args: &Args) -> Result<TcpListener> {
    Ok(TcpListener::bind(&args.listen).await?)
}

/// Create a database connection with migrations applied.
///
/// This function takes `Args` as input and attempts to parse the database connection string.
/// The connection string is then used to open a database connection. The migrator is called
/// to apply any pending migrations, and the connection is then returned.
///
/// # Errors
///
/// If the connection string is invalid, or if the connection cannot be established, or if the
/// migration fails, an error is returned.
async fn make_database(args: &Args) -> Result<DatabaseConnection> {
    // parse connection string
    let opt = ConnectOptions::new(&args.database);

    // open database connection
    let conn = Database::connect(opt).await?;

    // run migrations and return connection
    Ok({
        Migrator::up(&conn, None).await?;
        conn
    })
}

/// Creates a broadcast channel that can be used to signal shutdown to other tasks.
///
/// The returned receiver can be used to receive a shutdown signal. When the signal is
/// received, the task should shut down.
///
/// The shutdown signal is sent when either a CTRL-C signal is received, or a SIGTERM
/// signal is received.
fn make_shutdown_signal() -> broadcast::Receiver<()> {
    // create broadcast channel
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // spawn a task to listen for shutdown signals
    tokio::spawn(async move {
        // create a future that completes on CTRL-C
        let ctrl_c = async { signal::ctrl_c().await.unwrap() };

        // create a future that completes on SIGTERM
        let terminate = async {
            #[cfg(unix)]
            {
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("failed to install SIGTERM signal handler")
                    .recv()
                    .await
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await
            }
        };

        // select on either future
        select! {
            _ = ctrl_c => {}
            _ = terminate => {}
        }

        // broadcast shutdown signal
        shutdown_tx.send(()).unwrap();
    });

    // return broadcast receiver
    shutdown_rx
}
