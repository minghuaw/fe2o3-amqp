//! Error handling and recovery.
//!
//! Run a local broker on `amqp://localhost:5672` to execute this example. It
//! sends a message, classifies a failure with [`ErrorRecovery`], and closes
//! the link, session and connection while reporting the terminal outcomes.

use fe2o3_amqp::{
    connection::{Connection, ConnectionOutcome},
    link::{ErrorRecovery, LinkOutcome, Sender},
    session::{Session, SessionOutcome},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut connection =
        Connection::open("error-handling-example", "amqp://localhost:5672").await?;
    let mut session = Session::begin(&mut connection).await?;
    let mut sender = Sender::attach(&mut session, "rust-sender-1", "q1").await?;

    match sender.send("hello AMQP").await {
        Ok(outcome) => {
            // The delivery settled with this outcome
            println!("delivery settled: {outcome:?}");
        }
        Err(error) => {
            eprintln!("send failed: {error}");

            // Every link error tells the caller what to do next.
            match error.recovery() {
                // The link is still usable; the failure was scoped to the
                // operation.
                ErrorRecovery::UseLink => {}
                // The link is detached or was suspended by the peer; detach
                // (idempotent) and resume it.
                ErrorRecovery::ReattachLink => {
                    let (detached, outcome) = sender.detach().await.map_err(|(_, error)| error)?;
                    if outcome.is_closed() {
                        // A closed outcome is final; attach a new link instead.
                        println!("the peer closed the link: {outcome:?}");
                        sender = Sender::attach(&mut session, "rust-sender-1", "q1").await?;
                    } else {
                        sender = detached.resume().await?;
                    }
                }
                // The session stopped; obtain a new session (and connection)
                // and resume the link on it with `resume_on_session`.
                ErrorRecovery::ReconnectSession | ErrorRecovery::ReconnectConnection => {
                    println!("the session or connection stopped; resume on a new one");
                }
                // The link was destroyed or its state is unknown; attach a
                // new link.
                ErrorRecovery::NewLink => {}
                // `ErrorRecovery` is `#[non_exhaustive]`; treat future
                // classifications conservatively.
                _ => {}
            }
        }
    }

    // Closing completes the exchange; the outcome carries the peer's error, if
    // any, so a remote close is data rather than an error.
    let link_outcome: LinkOutcome = sender.close().await?;
    println!(
        "link closed: {link_outcome:?} (peer error: {:?})",
        link_outcome.remote_error()
    );

    let session_outcome: SessionOutcome = session.end().await?;
    println!("session ended: {session_outcome:?}");

    let connection_outcome: ConnectionOutcome = connection.close().await?;
    println!("connection closed: {connection_outcome:?}");

    Ok(())
}
