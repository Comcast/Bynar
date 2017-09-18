extern crate goji;
extern crate log;
extern crate serde_json;

use self::goji::{Credentials, Jira};
use self::goji::Error as GojiError;
use self::goji::issues::*;
use self::serde_json::value::Value;

/// Create a new JIRA support ticket and return the ticket ID associated with it
pub fn create_support_ticket(
    host: &str,
    user: &str,
    pass: &str,
    title: &str,
    description: &str,
    environment: &str,
) -> Result<String, GojiError> {
    let issue_description = CreateIssue {
        fields: Fields {
            assignee: Assignee { name: "Cloud_Services_Storage_SRE".to_string() },
            components: vec![Component { name: "Ceph".into() }],
            description: description.into(),
            environment: environment.into(),
            issuetype: IssueType { id: "3".into() },
            reporter: Assignee { name: user.to_string() },
            priority: Priority { id: "4".into() },
            project: Project { key: "PLATINF".into() },
            summary: title.into(),
        },
    };
    let jira: Jira = Jira::new(
        host.to_string(),
        Credentials::Basic(user.into(), pass.into()),
    )?;
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
    host: &str,
    user: &str,
    pass: &str,
    issue_id: &str,
) -> Result<bool, GojiError> {
    let jira: Jira = Jira::new(
        host.to_string(),
        Credentials::Basic(user.into(), pass.into()),
    )?;
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
