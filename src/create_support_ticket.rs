use super::ConfigSettings;
use goji::issues::*;
use goji::{Credentials, Jira};
use helpers::error::*;
use log::debug;
use serde_json::value::Value;

/// Create a new JIRA support ticket and return the ticket ID associated with it
pub fn create_support_ticket(
    settings: &ConfigSettings,
    title: &str,
    description: &str,
) -> BynarResult<String> {
    let issue_description = CreateIssue {
        fields: Fields {
            assignee: Assignee {
                name: settings.jira_ticket_assignee.clone(),
            },
            components: vec![Component {
                name: "Ceph".into(),
            }],
            description: description.into(),
            issuetype: IssueType {
                id: settings.jira_issue_type.clone(),
            },
            priority: Priority {
                id: settings.jira_priority.clone(),
            },
            project: Project {
                key: settings.jira_project_id.clone(),
            },
            summary: title.into(),
        },
    };
    let jira: Jira = match settings.proxy {
        Some(ref url) => {
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(url)?)
                .build()?;
            Jira::from_client(
                settings.jira_host.to_string(),
                Credentials::Basic(settings.jira_user.clone(), settings.jira_password.clone()),
                client,
            )?
        }
        None => Jira::new(
            settings.jira_host.clone().to_string(),
            Credentials::Basic(settings.jira_user.clone(), settings.jira_password.clone()),
        )?,
    };
    let issue = Issues::new(&jira);

    debug!(
        "Creating JIRA ticket with information: {:?}",
        issue_description
    );
    let results = issue.create(issue_description)?;
    Ok(results.id)
}

/// Check to see if a JIRA support ticket is marked as resolved
pub fn ticket_resolved(settings: &ConfigSettings, issue_id: &str) -> BynarResult<bool> {
    let jira: Jira = match settings.proxy {
        Some(ref url) => {
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(url)?)
                .build()?;
            Jira::from_client(
                settings.jira_host.to_string(),
                Credentials::Basic(settings.jira_user.clone(), settings.jira_password.clone()),
                client,
            )?
        }
        None => Jira::new(
            settings.jira_host.clone().to_string(),
            Credentials::Basic(settings.jira_user.clone(), settings.jira_password.clone()),
        )?,
    };
    let issue = Issues::new(&jira);
    debug!("Fetching issue: {} for resolution information", issue_id);
    let results = issue.get(issue_id)?;
    match results.fields.get("resolutiondate") {
        Some(Value::Null) => Ok(false),
        Some(Value::String(_)) => Ok(true),
        Some(_) => Ok(false),
        //resolutiondate doesn't exist
        None => Ok(false),
    }
}
