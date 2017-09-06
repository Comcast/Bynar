extern crate chrono;
extern crate goji;
extern crate log;

use self::chrono::DateTime;
use self::goji::{Credentials, Jira};
use self::goji::issues::*;

//TODO: This is just example code
pub fn create_support_ticket(
    host: String,
    user: String,
    pass: String,
    title: String,
    description: String,
) -> Result<(), String> {
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
    let jira = Jira::new(host, Credentials::Basic(user, pass)).map_err(
        |e| {
            e.to_string()
        },
    )?;
    let issue = Issues::new(&jira);

    let results = issue.create(issue_description);
    println!("Result: {:?}", results);
}

pub fn ticket_resolved(
    host: String,
    user: String,
    pass: String,
    issue_id: String,
) -> Result<Option<DateTime>, String> {
    let jira = Jira::new(host, Credentials::Basic(user, pass)).map_err(
        |e| {
            e.to_string()
        },
    )?;
    let issue = Issues::new(&jira);

    let results = issue.get(issue_id).unwrap();
    let resolved_date = results.fields.get("resolutiondate").unwrap();

    let date_str = resolved_date.as_str().unwrap().to_string();
    let date_time = DateTime::parse_from_str(&date_str, "%Y-%m-%dT%T%.3f%z")
        .map_err(|e| e.to_string())?;
    Ok(date_time_str)
}
