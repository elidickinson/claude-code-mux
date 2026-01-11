pub mod auth;
pub mod cli;
pub mod message_tracing;
pub mod models;
pub mod pid;
pub mod providers;
pub mod router;
pub mod server;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}