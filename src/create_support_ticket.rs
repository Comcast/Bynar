extern crate goji;
extern crate log;
extern crate reqwest;
extern crate serde_json;

use self::goji::{Credentials, Jira};
use self::goji::Error as GojiError;
use self::goji::issues::*;
use self::serde_json::value::Value;
use super::ConfigSettings;

/// Create a new JIRA support ticket and return the ticket ID associated with it
pub fn create_support_ticket(
    settings: &ConfigSettings,
    title: &str,
    description: &str,
    environment: &str,
) -> Result<String, GojiError> {
    let issue_description = CreateIssue {
        fields: Fields {
            assignee: Assignee {
                name: settings.jira_ticket_assignee.clone(),
            },
            components: vec![
                Component {
                    name: "Ceph".into(),
                },
            ],
            description: description.into(),
            environment: environment.into(),
            issuetype: IssueType {
                id: settings.jira_issue_type.clone(),
            },
            reporter: Assignee {
                name: settings.jira_user.clone(),
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
                Credentials::Basic(
                    settings.jira_user.clone().into(),
                    settings.jira_password.clone().into(),
                ),
                client,
            )?
        }
        None => Jira::new(
            settings.jira_host.clone().to_string(),
            Credentials::Basic(
                settings.jira_user.clone().into(),
                settings.jira_password.clone().into(),
            ),
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
pub fn ticket_resolved(settings: &ConfigSettings, issue_id: &str) -> Result<bool, GojiError> {
    let jira: Jira = match settings.proxy {
        Some(ref url) => {
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(url)?)
                .build()?;
            Jira::from_client(
                settings.jira_host.to_string(),
                Credentials::Basic(
                    settings.jira_user.clone().into(),
                    settings.jira_password.clone().into(),
                ),
                client,
            )?
        }
        None => Jira::new(
            settings.jira_host.clone().to_string(),
            Credentials::Basic(
                settings.jira_user.clone().into(),
                settings.jira_password.clone().into(),
            ),
        )?,
    };
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
