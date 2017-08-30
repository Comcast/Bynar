extern crate goji;
extern crate hyper;
extern crate hyper_openssl;

use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_openssl::OpensslClient;
use goji::{Credentials, Jira};

use std::env;

//TODO: This is just example code
pub fn create_support_ticket() {
    if let (Ok(host), Ok(user), Ok(pass)) =
        (
            env::var("JIRA_HOST"),
            env::var("JIRA_USER"),
            env::var("JIRA_PASS"),
        )
    {
        let query = env::args().nth(1).unwrap();

        let ssl = OpensslClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        let client = Client::with_connector(connector);

        let jira = Jira::new(host, Credentials::Basic(user, pass), &client);

        let results = jira.search().list(query, &Default::default());
        for issue in results.unwrap().issues {
            println!("{:#?}", issue)
        }
    }
}
