extern crate goji;
use self::goji::{Credentials, Jira};

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
        let jira = Jira::new(host, Credentials::Basic(user, pass)).unwrap();

        let results = jira.search().list(query, &Default::default());
        for issue in results.unwrap().issues {
            println!("{:#?}", issue)
        }
    }
}
