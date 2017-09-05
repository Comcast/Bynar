extern crate goji;
extern crate log;

use self::goji::{Credentials, Jira};
use self::goji::issues::*;

//TODO: This is just example code
pub fn create_support_ticket(
    host: String,
    user: String,
    pass: String,
    title: String,
    description: String,
) {
    let issue_description = CreateIssue {
        fields: Fields {
            assignee: Assignee { name: user.clone() },
            components: vec![Component { name: "Ceph".into() }],
            description: description,
            issuetype: IssueType { id: "3".into() },
            reporter: Assignee { name: user.clone() },
            priority: Priority { id: "4".into() },
            project: Project { key: "PLATINF".into() },
            summary: title,
        },
    };
    let jira = Jira::new(host, Credentials::Basic(user, pass)).unwrap();
    let issue = Issues::new(&jira);

    let results = issue.create(issue_description);
    println!("Result: {:?}", results);
}
