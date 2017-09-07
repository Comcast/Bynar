extern crate goji;
extern crate log;
extern crate serde_json;

use self::goji::{Credentials, Jira};
use self::goji::Error as GojiError;
use self::goji::issues::*;
use self::serde_json::value::Value;

/// Create a new JIRA support ticket and return the ticket ID associated with it
pub fn create_support_ticket(
    host: String,
    user: String,
    pass: String,
    title: String,
    description: String,
) -> Result<String, GojiError> {
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
    let jira = Jira::new(host, Credentials::Basic(user, pass))?;
    let issue = Issues::new(&jira);

    debug!(
        "Creating JIRA ticket with information: {:?}",
        issue_description
    );
    let results = issue.create(issue_description)?;
    Ok(results.id)
}

/// Check to see if a JIRA support ticket is marked as resolved
pub fn ticket_resolved(
    host: String,
    user: String,
    pass: String,
    issue_id: String,
) -> Result<bool, GojiError> {
    let jira = Jira::new(host, Credentials::Basic(user, pass))?;
    let issue = Issues::new(&jira);
    debug!("Fetching issue: {} for resolution information", issue_id);
    let results = issue.get(issue_id)?;
    match results.fields.get("resolutiondate") {
        Some(v) => {
            match v {
                //resolutiondate is null
                &Value::Null => Ok(false),
                //resolutiondate is set.
                &Value::String(_) => Ok(true),
                _ => Ok(false),
            }
        }
        //resolutiondate doesn't exist
        None => Ok(false),
    }
}
