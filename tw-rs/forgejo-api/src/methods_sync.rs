use super::structs::*;
use crate::generated::endpoints;
use crate::Endpoint;
use crate::Request;
use bytes::Bytes;
use std::collections::BTreeMap;

impl crate::sync::Forgejo {
    /// Returns the instance's Actor
    pub fn activitypub_instance_actor(
        &self,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubInstanceActor, ActivityPub> {
        endpoints::ActivitypubInstanceActor {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Send to the inbox
    pub fn activitypub_instance_actor_inbox(
        &self,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubInstanceActorInbox, ()> {
        endpoints::ActivitypubInstanceActorInbox {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Display the outbox (always empty)
    pub fn activitypub_instance_actor_outbox(
        &self,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubInstanceActorOutbox, Bytes> {
        endpoints::ActivitypubInstanceActorOutbox {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns the Repository actor for a repo
    ///
    /// - `repository-id`: repository ID of the repo
    pub fn activitypub_repository(
        &self,
        repository_id: i64,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubRepository, ActivityPub> {
        endpoints::ActivitypubRepository { repository_id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Send to the inbox
    ///
    /// - `repository-id`: repository ID of the repo
    /// - `body`: See [`ForgeLike`]
    pub fn activitypub_repository_inbox(
        &self,
        repository_id: i64,
        body: ForgeLike,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubRepositoryInbox, ()> {
        endpoints::ActivitypubRepositoryInbox {
            repository_id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Display the outbox
    ///
    /// - `repository-id`: repository ID of the repo
    pub fn activitypub_repository_outbox(
        &self,
        repository_id: i64,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubRepositoryOutbox, Bytes> {
        endpoints::ActivitypubRepositoryOutbox { repository_id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns the Person actor for a user
    ///
    /// - `user-id`: user ID of the user
    pub fn activitypub_person(
        &self,
        user_id: i64,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubPerson, ActivityPub> {
        endpoints::ActivitypubPerson { user_id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a specific activity object of the user
    ///
    /// - `user-id`: user ID of the user
    /// - `activity-id`: activity ID of the sought activity
    pub fn activitypub_person_activity_note(
        &self,
        user_id: u32,
        activity_id: u32,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubPersonActivityNote, ActivityPub> {
        endpoints::ActivitypubPersonActivityNote {
            user_id,
            activity_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a specific activity of the user
    ///
    /// - `user-id`: user ID of the user
    /// - `activity-id`: activity ID of the sought activity
    pub fn activitypub_person_activity(
        &self,
        user_id: u32,
        activity_id: u32,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubPersonActivity, ActivityPub> {
        endpoints::ActivitypubPersonActivity {
            user_id,
            activity_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Send to the inbox
    ///
    /// - `user-id`: user ID of the user
    pub fn activitypub_person_inbox(
        &self,
        user_id: i64,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubPersonInbox, ()> {
        endpoints::ActivitypubPersonInbox { user_id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the user's recorded activity
    ///
    /// - `user-id`: user ID of the user
    pub fn activitypub_person_feed(
        &self,
        user_id: u32,
    ) -> crate::sync::Request<'_, endpoints::ActivitypubPersonFeed, ForgeOutbox> {
        endpoints::ActivitypubPersonFeed { user_id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List cron tasks
    ///
    pub fn admin_cron_list(
        &self,
    ) -> crate::sync::Request<'_, endpoints::AdminCronList, (CronListHeaders, Vec<Cron>)> {
        endpoints::AdminCronList {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Run cron task
    ///
    /// - `task`: task to run
    pub fn admin_cron_run(
        &self,
        task: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminCronRun<'_>, ()> {
        endpoints::AdminCronRun { task }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List all users' email addresses
    ///
    pub fn admin_get_all_emails(
        &self,
    ) -> crate::sync::Request<'_, endpoints::AdminGetAllEmails, Vec<Email>> {
        endpoints::AdminGetAllEmails {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search users' email addresses
    ///
    pub fn admin_search_emails(
        &self,
        query: AdminSearchEmailsQuery,
    ) -> crate::sync::Request<'_, endpoints::AdminSearchEmails, Vec<Email>> {
        endpoints::AdminSearchEmails { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List global (system) webhooks
    ///
    pub fn admin_list_hooks(
        &self,
    ) -> crate::sync::Request<'_, endpoints::AdminListHooks, Vec<Hook>> {
        endpoints::AdminListHooks {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a hook
    ///
    /// - `body`: See [`CreateHookOption`]
    pub fn admin_create_hook(
        &self,
        body: CreateHookOption,
    ) -> crate::sync::Request<'_, endpoints::AdminCreateHook, Hook> {
        endpoints::AdminCreateHook { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a hook
    ///
    /// - `id`: id of the hook to get
    pub fn admin_get_hook(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::AdminGetHook, Hook> {
        endpoints::AdminGetHook { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a hook
    ///
    /// - `id`: id of the hook to delete
    pub fn admin_delete_hook(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteHook, ()> {
        endpoints::AdminDeleteHook { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a hook
    ///
    /// - `id`: id of the hook to update
    /// - `body`: See [`EditHookOption`]
    pub fn admin_edit_hook(
        &self,
        id: i64,
        body: EditHookOption,
    ) -> crate::sync::Request<'_, endpoints::AdminEditHook, Hook> {
        endpoints::AdminEditHook { id, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List all organizations
    ///
    pub fn admin_get_all_orgs(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::AdminGetAllOrgs,
        (OrganizationListHeaders, Vec<Organization>),
    > {
        endpoints::AdminGetAllOrgs {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the available quota groups
    pub fn admin_list_quota_groups(
        &self,
    ) -> crate::sync::Request<'_, endpoints::AdminListQuotaGroups, Vec<QuotaGroup>> {
        endpoints::AdminListQuotaGroups {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a new quota group
    ///
    /// - `group`: Definition of the quota group

    ///   See [`CreateQuotaGroupOptions`]
    pub fn admin_create_quota_group(
        &self,
        group: CreateQuotaGroupOptions,
    ) -> crate::sync::Request<'_, endpoints::AdminCreateQuotaGroup, QuotaGroup> {
        endpoints::AdminCreateQuotaGroup { body: group }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get information about the quota group
    ///
    /// - `quotagroup`: quota group to query
    pub fn admin_get_quota_group(
        &self,
        quotagroup: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminGetQuotaGroup<'_>, QuotaGroup> {
        endpoints::AdminGetQuotaGroup { quotagroup }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a quota group
    ///
    /// - `quotagroup`: quota group to delete
    pub fn admin_delete_quota_group(
        &self,
        quotagroup: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteQuotaGroup<'_>, ()> {
        endpoints::AdminDeleteQuotaGroup { quotagroup }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Adds a rule to a quota group
    ///
    /// - `quotagroup`: quota group to add a rule to
    /// - `quotarule`: the name of the quota rule to add to the group
    pub fn admin_add_rule_to_quota_group(
        &self,
        quotagroup: &str,
        quotarule: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminAddRuleToQuotaGroup<'_>, ()> {
        endpoints::AdminAddRuleToQuotaGroup {
            quotagroup,
            quotarule,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Removes a rule from a quota group
    ///
    /// - `quotagroup`: quota group to remove a rule from
    /// - `quotarule`: the name of the quota rule to remove from the group
    pub fn admin_remove_rule_from_quota_group(
        &self,
        quotagroup: &str,
        quotarule: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminRemoveRuleFromQuotaGroup<'_>, ()> {
        endpoints::AdminRemoveRuleFromQuotaGroup {
            quotagroup,
            quotarule,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List users in a quota group
    ///
    /// - `quotagroup`: quota group to list members of
    pub fn admin_list_users_in_quota_group(
        &self,
        quotagroup: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::AdminListUsersInQuotaGroup<'_>,
        (UserListHeaders, Vec<User>),
    > {
        endpoints::AdminListUsersInQuotaGroup { quotagroup }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a user to a quota group
    ///
    /// - `quotagroup`: quota group to add the user to
    /// - `username`: username of the user to add to the quota group
    pub fn admin_add_user_to_quota_group(
        &self,
        quotagroup: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminAddUserToQuotaGroup<'_>, ()> {
        endpoints::AdminAddUserToQuotaGroup {
            quotagroup,
            username,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove a user from a quota group
    ///
    /// - `quotagroup`: quota group to remove a user from
    /// - `username`: username of the user to remove from the quota group
    pub fn admin_remove_user_from_quota_group(
        &self,
        quotagroup: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminRemoveUserFromQuotaGroup<'_>, ()> {
        endpoints::AdminRemoveUserFromQuotaGroup {
            quotagroup,
            username,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List the available quota rules
    pub fn admin_list_quota_rules(
        &self,
    ) -> crate::sync::Request<'_, endpoints::AdminListQuotaRules, Vec<QuotaRuleInfo>> {
        endpoints::AdminListQuotaRules {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a new quota rule
    ///
    /// - `rule`: Definition of the quota rule

    ///   See [`CreateQuotaRuleOptions`]
    pub fn admin_create_quota_rule(
        &self,
        rule: CreateQuotaRuleOptions,
    ) -> crate::sync::Request<'_, endpoints::AdminCreateQuotaRule, QuotaRuleInfo> {
        endpoints::AdminCreateQuotaRule { body: rule }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get information about a quota rule
    ///
    /// - `quotarule`: quota rule to query
    pub fn admin_get_quota_rule(
        &self,
        quotarule: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminGetQuotaRule<'_>, QuotaRuleInfo> {
        endpoints::AdminGetQuotaRule { quotarule }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Deletes a quota rule
    ///
    /// - `quotarule`: quota rule to delete
    pub fn admin_delete_quota_rule(
        &self,
        quotarule: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteQuotaRule<'_>, ()> {
        endpoints::AdminDeleteQuotaRule { quotarule }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Change an existing quota rule
    ///
    /// - `quotarule`: Quota rule to change
    /// - `rule`: See [`EditQuotaRuleOptions`]
    pub fn admin_edit_quota_rule(
        &self,
        quotarule: &str,
        rule: EditQuotaRuleOptions,
    ) -> crate::sync::Request<'_, endpoints::AdminEditQuotaRule<'_>, QuotaRuleInfo> {
        endpoints::AdminEditQuotaRule {
            quotarule,
            body: rule,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Search action jobs according filter conditions
    ///
    pub fn admin_search_run_jobs(
        &self,
        query: AdminSearchRunJobsQuery,
    ) -> crate::sync::Request<'_, endpoints::AdminSearchRunJobs, Vec<ActionRunJob>> {
        endpoints::AdminSearchRunJobs { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an global actions runner registration token
    pub fn admin_get_runner_registration_token(
        &self,
    ) -> crate::sync::Request<'_, endpoints::AdminGetRunnerRegistrationToken, RegistrationToken>
    {
        endpoints::AdminGetRunnerRegistrationToken {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List unadopted repositories
    ///
    pub fn admin_unadopted_list(
        &self,
        query: AdminUnadoptedListQuery,
    ) -> crate::sync::Request<'_, endpoints::AdminUnadoptedList, Vec<String>> {
        endpoints::AdminUnadoptedList { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Adopt unadopted files as a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn admin_adopt_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminAdoptRepository<'_>, ()> {
        endpoints::AdminAdoptRepository { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete unadopted files
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn admin_delete_unadopted_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteUnadoptedRepository<'_>, ()> {
        endpoints::AdminDeleteUnadoptedRepository { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search users according filter conditions
    ///
    pub fn admin_search_users(
        &self,
        query: AdminSearchUsersQuery,
    ) -> crate::sync::Request<'_, endpoints::AdminSearchUsers, (UserListHeaders, Vec<User>)> {
        endpoints::AdminSearchUsers { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a user account
    ///
    /// - `body`: See [`CreateUserOption`]
    pub fn admin_create_user(
        &self,
        body: CreateUserOption,
    ) -> crate::sync::Request<'_, endpoints::AdminCreateUser, User> {
        endpoints::AdminCreateUser { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete user account
    ///
    /// - `username`: username of user to delete
    pub fn admin_delete_user(
        &self,
        username: &str,
        query: AdminDeleteUserQuery,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteUser<'_>, ()> {
        endpoints::AdminDeleteUser { username, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit an existing user
    ///
    /// - `username`: username of user to edit
    /// - `body`: See [`EditUserOption`]
    pub fn admin_edit_user(
        &self,
        username: &str,
        body: EditUserOption,
    ) -> crate::sync::Request<'_, endpoints::AdminEditUser<'_>, User> {
        endpoints::AdminEditUser {
            username,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List all email addresses for a user
    ///
    /// - `username`: username of user to get email addresses of
    pub fn admin_list_user_emails(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminListUserEmails<'_>, Vec<Email>> {
        endpoints::AdminListUserEmails { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete email addresses from a user's account
    ///
    /// - `username`: username of user to delete email addresses from
    /// - `body`: See [`DeleteEmailOption`]
    pub fn admin_delete_user_emails(
        &self,
        username: &str,
        body: DeleteEmailOption,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteUserEmails<'_>, ()> {
        endpoints::AdminDeleteUserEmails {
            username,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add an SSH public key to user's account
    ///
    /// - `username`: username of the user
    /// - `key`: See [`CreateKeyOption`]
    pub fn admin_create_public_key(
        &self,
        username: &str,
        key: CreateKeyOption,
    ) -> crate::sync::Request<'_, endpoints::AdminCreatePublicKey<'_>, PublicKey> {
        endpoints::AdminCreatePublicKey {
            username,
            body: key,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove a public key from user's account
    ///
    /// - `username`: username of user
    /// - `id`: id of the key to delete
    pub fn admin_delete_user_public_key(
        &self,
        username: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::AdminDeleteUserPublicKey<'_>, ()> {
        endpoints::AdminDeleteUserPublicKey { username, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create an organization
    ///
    /// - `username`: username of the user that will own the created organization
    /// - `organization`: See [`CreateOrgOption`]
    pub fn admin_create_org(
        &self,
        username: &str,
        organization: CreateOrgOption,
    ) -> crate::sync::Request<'_, endpoints::AdminCreateOrg<'_>, Organization> {
        endpoints::AdminCreateOrg {
            username,
            body: organization,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get the user's quota info
    ///
    /// - `username`: username of user to query
    pub fn admin_get_user_quota(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::AdminGetUserQuota<'_>, QuotaInfo> {
        endpoints::AdminGetUserQuota { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Set the user's quota groups to a given list.
    ///
    /// - `username`: username of the user to modify the quota groups from
    /// - `groups`: list of groups that the user should be a member of

    ///   See [`SetUserQuotaGroupsOptions`]
    pub fn admin_set_user_quota_groups(
        &self,
        username: &str,
        groups: SetUserQuotaGroupsOptions,
    ) -> crate::sync::Request<'_, endpoints::AdminSetUserQuotaGroups<'_>, ()> {
        endpoints::AdminSetUserQuotaGroups {
            username,
            body: groups,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Rename a user
    ///
    /// - `username`: existing username of user
    /// - `body`: See [`RenameUserOption`]
    pub fn admin_rename_user(
        &self,
        username: &str,
        body: RenameUserOption,
    ) -> crate::sync::Request<'_, endpoints::AdminRenameUser<'_>, ()> {
        endpoints::AdminRenameUser {
            username,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a repository on behalf of a user
    ///
    /// - `username`: username of the user. This user will own the created repository
    /// - `repository`: See [`CreateRepoOption`]
    pub fn admin_create_repo(
        &self,
        username: &str,
        repository: CreateRepoOption,
    ) -> crate::sync::Request<'_, endpoints::AdminCreateRepo<'_>, Repository> {
        endpoints::AdminCreateRepo {
            username,
            body: repository,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Returns a list of all gitignore templates
    pub fn list_gitignores_templates(
        &self,
    ) -> crate::sync::Request<'_, endpoints::ListGitignoresTemplates, Vec<String>> {
        endpoints::ListGitignoresTemplates {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns information about a gitignore template
    ///
    /// - `name`: name of the template
    pub fn get_gitignore_template_info(
        &self,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::GetGitignoreTemplateInfo<'_>, GitignoreTemplateInfo>
    {
        endpoints::GetGitignoreTemplateInfo { name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns a list of all label templates
    pub fn list_label_templates(
        &self,
    ) -> crate::sync::Request<'_, endpoints::ListLabelTemplates, Vec<String>> {
        endpoints::ListLabelTemplates {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns all labels in a template
    ///
    /// - `name`: name of the template
    pub fn get_label_template_info(
        &self,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::GetLabelTemplateInfo<'_>, Vec<LabelTemplate>> {
        endpoints::GetLabelTemplateInfo { name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns a list of all license templates
    pub fn list_license_templates(
        &self,
    ) -> crate::sync::Request<'_, endpoints::ListLicenseTemplates, Vec<LicensesTemplateListEntry>>
    {
        endpoints::ListLicenseTemplates {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns information about a license template
    ///
    /// - `name`: name of the license
    pub fn get_license_template_info(
        &self,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::GetLicenseTemplateInfo<'_>, LicenseTemplateInfo> {
        endpoints::GetLicenseTemplateInfo { name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Render a markdown document as HTML
    ///
    /// - `body`: See [`MarkdownOption`]
    pub fn render_markdown(
        &self,
        body: MarkdownOption,
    ) -> crate::sync::Request<'_, endpoints::RenderMarkdown, String> {
        endpoints::RenderMarkdown { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Render raw markdown as HTML
    ///
    /// - `body`: Request body to render

    ///   See [`String`]
    pub fn render_markdown_raw(
        &self,
        body: String,
    ) -> crate::sync::Request<'_, endpoints::RenderMarkdownRaw, String> {
        endpoints::RenderMarkdownRaw { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Render a markup document as HTML
    ///
    /// - `body`: See [`MarkupOption`]
    pub fn render_markup(
        &self,
        body: MarkupOption,
    ) -> crate::sync::Request<'_, endpoints::RenderMarkup, String> {
        endpoints::RenderMarkup { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns the nodeinfo of the Forgejo application
    pub fn get_node_info(&self) -> crate::sync::Request<'_, endpoints::GetNodeInfo, NodeInfo> {
        endpoints::GetNodeInfo {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List users's notification threads
    ///
    pub fn notify_get_list(
        &self,
        query: NotifyGetListQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::NotifyGetList,
        (NotificationThreadListHeaders, Vec<NotificationThread>),
    > {
        endpoints::NotifyGetList { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Mark notification threads as read, pinned or unread
    ///
    pub fn notify_read_list(
        &self,
        query: NotifyReadListQuery,
    ) -> crate::sync::Request<'_, endpoints::NotifyReadList, Vec<NotificationThread>> {
        endpoints::NotifyReadList { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if unread notifications exist
    pub fn notify_new_available(
        &self,
    ) -> crate::sync::Request<'_, endpoints::NotifyNewAvailable, NotificationCount> {
        endpoints::NotifyNewAvailable {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get notification thread by ID
    ///
    /// - `id`: id of notification thread
    pub fn notify_get_thread(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::NotifyGetThread, NotificationThread> {
        endpoints::NotifyGetThread { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Mark notification thread as read by ID
    ///
    /// - `id`: id of notification thread
    pub fn notify_read_thread(
        &self,
        id: i64,
        query: NotifyReadThreadQuery,
    ) -> crate::sync::Request<'_, endpoints::NotifyReadThread, NotificationThread> {
        endpoints::NotifyReadThread { id, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a repository in an organization
    ///
    /// - `org`: name of organization
    /// - `body`: See [`CreateRepoOption`]
    pub fn create_org_repo_deprecated(
        &self,
        org: &str,
        body: CreateRepoOption,
    ) -> crate::sync::Request<'_, endpoints::CreateOrgRepoDeprecated<'_>, Repository> {
        endpoints::CreateOrgRepoDeprecated { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List all organizations
    ///
    pub fn org_get_all(
        &self,
    ) -> crate::sync::Request<'_, endpoints::OrgGetAll, (OrganizationListHeaders, Vec<Organization>)>
    {
        endpoints::OrgGetAll {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create an organization
    ///
    /// - `organization`: See [`CreateOrgOption`]
    pub fn org_create(
        &self,
        organization: CreateOrgOption,
    ) -> crate::sync::Request<'_, endpoints::OrgCreate, Organization> {
        endpoints::OrgCreate { body: organization }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an organization
    ///
    /// - `org`: name of the organization to get
    pub fn org_get(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgGet<'_>, Organization> {
        endpoints::OrgGet { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete an organization
    ///
    /// - `org`: organization that is to be deleted
    pub fn org_delete(&self, org: &str) -> crate::sync::Request<'_, endpoints::OrgDelete<'_>, ()> {
        endpoints::OrgDelete { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit an organization
    ///
    /// - `org`: name of the organization to edit
    /// - `body`: See [`EditOrgOption`]
    pub fn org_edit(
        &self,
        org: &str,
        body: EditOrgOption,
    ) -> crate::sync::Request<'_, endpoints::OrgEdit<'_>, Organization> {
        endpoints::OrgEdit { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search for organization's action jobs according filter conditions
    ///
    /// - `org`: name of the organization
    pub fn org_search_run_jobs(
        &self,
        org: &str,
        query: OrgSearchRunJobsQuery,
    ) -> crate::sync::Request<'_, endpoints::OrgSearchRunJobs<'_>, Vec<ActionRunJob>> {
        endpoints::OrgSearchRunJobs { org, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an organization's actions runner registration token
    ///
    /// - `org`: name of the organization
    pub fn org_get_runner_registration_token(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgGetRunnerRegistrationToken<'_>, RegistrationToken>
    {
        endpoints::OrgGetRunnerRegistrationToken { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List actions secrets of an organization
    ///
    /// - `org`: name of the organization
    pub fn org_list_actions_secrets(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListActionsSecrets<'_>,
        (SecretListHeaders, Vec<Secret>),
    > {
        endpoints::OrgListActionsSecrets { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create or Update a secret value in an organization
    ///
    /// - `org`: name of organization
    /// - `secretname`: name of the secret
    /// - `body`: See [`CreateOrUpdateSecretOption`]
    pub fn update_org_secret(
        &self,
        org: &str,
        secretname: &str,
        body: CreateOrUpdateSecretOption,
    ) -> crate::sync::Request<'_, endpoints::UpdateOrgSecret<'_>, ()> {
        endpoints::UpdateOrgSecret {
            org,
            secretname,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a secret in an organization
    ///
    /// - `org`: name of organization
    /// - `secretname`: name of the secret
    pub fn delete_org_secret(
        &self,
        org: &str,
        secretname: &str,
    ) -> crate::sync::Request<'_, endpoints::DeleteOrgSecret<'_>, ()> {
        endpoints::DeleteOrgSecret { org, secretname }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List variables of an organization
    ///
    /// - `org`: name of the organization
    pub fn get_org_variables_list(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::GetOrgVariablesList<'_>,
        (VariableListHeaders, Vec<ActionVariable>),
    > {
        endpoints::GetOrgVariablesList { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get organization's variable by name
    ///
    /// - `org`: name of the organization
    /// - `variablename`: name of the variable
    pub fn get_org_variable(
        &self,
        org: &str,
        variablename: &str,
    ) -> crate::sync::Request<'_, endpoints::GetOrgVariable<'_>, ActionVariable> {
        endpoints::GetOrgVariable { org, variablename }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update variable in organization
    ///
    /// - `org`: name of the organization
    /// - `variablename`: name of the variable
    /// - `body`: See [`UpdateVariableOption`]
    pub fn update_org_variable(
        &self,
        org: &str,
        variablename: &str,
        body: UpdateVariableOption,
    ) -> crate::sync::Request<'_, endpoints::UpdateOrgVariable<'_>, ()> {
        endpoints::UpdateOrgVariable {
            org,
            variablename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a new variable in organization
    ///
    /// - `org`: name of the organization
    /// - `variablename`: name of the variable
    /// - `body`: See [`CreateVariableOption`]
    pub fn create_org_variable(
        &self,
        org: &str,
        variablename: &str,
        body: CreateVariableOption,
    ) -> crate::sync::Request<'_, endpoints::CreateOrgVariable<'_>, ()> {
        endpoints::CreateOrgVariable {
            org,
            variablename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete organization's variable by name
    ///
    /// - `org`: name of the organization
    /// - `variablename`: name of the variable
    pub fn delete_org_variable(
        &self,
        org: &str,
        variablename: &str,
    ) -> crate::sync::Request<'_, endpoints::DeleteOrgVariable<'_>, ()> {
        endpoints::DeleteOrgVariable { org, variablename }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List an organization's activity feeds
    ///
    /// - `org`: name of the org
    pub fn org_list_activity_feeds(
        &self,
        org: &str,
        query: OrgListActivityFeedsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListActivityFeeds<'_>,
        (ActivityFeedsListHeaders, Vec<Activity>),
    > {
        endpoints::OrgListActivityFeeds { org, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update an organization's avatar
    ///
    /// - `org`: name of the organization
    /// - `body`: See [`UpdateUserAvatarOption`]
    pub fn org_update_avatar(
        &self,
        org: &str,
        body: UpdateUserAvatarOption,
    ) -> crate::sync::Request<'_, endpoints::OrgUpdateAvatar<'_>, ()> {
        endpoints::OrgUpdateAvatar { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete an organization's avatar. It will be replaced by a default one
    ///
    /// - `org`: name of the organization
    pub fn org_delete_avatar(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgDeleteAvatar<'_>, ()> {
        endpoints::OrgDeleteAvatar { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Blocks a user from the organization
    ///
    /// - `org`: name of the org
    /// - `username`: username of the user
    pub fn org_block_user(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgBlockUser<'_>, ()> {
        endpoints::OrgBlockUser { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List an organization's webhooks
    ///
    /// - `org`: name of the organization
    pub fn org_list_hooks(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListHooks<'_>, Vec<Hook>> {
        endpoints::OrgListHooks { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a hook
    ///
    /// - `org`: name of the organization
    /// - `body`: See [`CreateHookOption`]
    pub fn org_create_hook(
        &self,
        org: &str,
        body: CreateHookOption,
    ) -> crate::sync::Request<'_, endpoints::OrgCreateHook<'_>, Hook> {
        endpoints::OrgCreateHook { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a hook
    ///
    /// - `org`: name of the organization
    /// - `id`: id of the hook to get
    pub fn org_get_hook(
        &self,
        org: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::OrgGetHook<'_>, Hook> {
        endpoints::OrgGetHook { org, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a hook
    ///
    /// - `org`: name of the organization
    /// - `id`: id of the hook to delete
    pub fn org_delete_hook(
        &self,
        org: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::OrgDeleteHook<'_>, ()> {
        endpoints::OrgDeleteHook { org, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a hook
    ///
    /// - `org`: name of the organization
    /// - `id`: id of the hook to update
    /// - `body`: See [`EditHookOption`]
    pub fn org_edit_hook(
        &self,
        org: &str,
        id: i64,
        body: EditHookOption,
    ) -> crate::sync::Request<'_, endpoints::OrgEditHook<'_>, Hook> {
        endpoints::OrgEditHook {
            org,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List an organization's labels
    ///
    /// - `org`: name of the organization
    pub fn org_list_labels(
        &self,
        org: &str,
        query: OrgListLabelsQuery,
    ) -> crate::sync::Request<'_, endpoints::OrgListLabels<'_>, (LabelListHeaders, Vec<Label>)>
    {
        endpoints::OrgListLabels { org, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a label for an organization
    ///
    /// - `org`: name of the organization
    /// - `body`: See [`CreateLabelOption`]
    pub fn org_create_label(
        &self,
        org: &str,
        body: CreateLabelOption,
    ) -> crate::sync::Request<'_, endpoints::OrgCreateLabel<'_>, Label> {
        endpoints::OrgCreateLabel { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a single label
    ///
    /// - `org`: name of the organization
    /// - `id`: id of the label to get
    pub fn org_get_label(
        &self,
        org: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::OrgGetLabel<'_>, Label> {
        endpoints::OrgGetLabel { org, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a label
    ///
    /// - `org`: name of the organization
    /// - `id`: id of the label to delete
    pub fn org_delete_label(
        &self,
        org: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::OrgDeleteLabel<'_>, ()> {
        endpoints::OrgDeleteLabel { org, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a label
    ///
    /// - `org`: name of the organization
    /// - `id`: id of the label to edit
    /// - `body`: See [`EditLabelOption`]
    pub fn org_edit_label(
        &self,
        org: &str,
        id: i64,
        body: EditLabelOption,
    ) -> crate::sync::Request<'_, endpoints::OrgEditLabel<'_>, Label> {
        endpoints::OrgEditLabel {
            org,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List the organization's blocked users
    ///
    /// - `org`: name of the org
    pub fn org_list_blocked_users(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListBlockedUsers<'_>,
        (BlockedUserListHeaders, Vec<BlockedUser>),
    > {
        endpoints::OrgListBlockedUsers { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List an organization's members
    ///
    /// - `org`: name of the organization
    pub fn org_list_members(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListMembers<'_>, (UserListHeaders, Vec<User>)> {
        endpoints::OrgListMembers { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if a user is a member of an organization
    ///
    /// - `org`: name of the organization
    /// - `username`: username of the user
    pub fn org_is_member(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgIsMember<'_>, ()> {
        endpoints::OrgIsMember { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Remove a member from an organization
    ///
    /// - `org`: name of the organization
    /// - `username`: username of the user
    pub fn org_delete_member(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgDeleteMember<'_>, ()> {
        endpoints::OrgDeleteMember { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List an organization's public members
    ///
    /// - `org`: name of the organization
    pub fn org_list_public_members(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListPublicMembers<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::OrgListPublicMembers { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if a user is a public member of an organization
    ///
    /// - `org`: name of the organization
    /// - `username`: username of the user
    pub fn org_is_public_member(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgIsPublicMember<'_>, ()> {
        endpoints::OrgIsPublicMember { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Publicize a user's membership
    ///
    /// - `org`: name of the organization
    /// - `username`: username of the user
    pub fn org_publicize_member(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgPublicizeMember<'_>, ()> {
        endpoints::OrgPublicizeMember { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Conceal a user's membership
    ///
    /// - `org`: name of the organization
    /// - `username`: username of the user
    pub fn org_conceal_member(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgConcealMember<'_>, ()> {
        endpoints::OrgConcealMember { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get quota information for an organization
    ///
    /// - `org`: name of the organization
    pub fn org_get_quota(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgGetQuota<'_>, QuotaInfo> {
        endpoints::OrgGetQuota { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the artifacts affecting the organization's quota
    ///
    /// - `org`: name of the organization
    pub fn org_list_quota_artifacts(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListQuotaArtifacts<'_>,
        (QuotaUsedArtifactListHeaders, Vec<QuotaUsedArtifact>),
    > {
        endpoints::OrgListQuotaArtifacts { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the attachments affecting the organization's quota
    ///
    /// - `org`: name of the organization
    pub fn org_list_quota_attachments(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListQuotaAttachments<'_>,
        (QuotaUsedAttachmentListHeaders, Vec<QuotaUsedAttachment>),
    > {
        endpoints::OrgListQuotaAttachments { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if the organization is over quota for a given subject
    ///
    /// - `org`: name of the organization
    pub fn org_check_quota(
        &self,
        org: &str,
        query: OrgCheckQuotaQuery,
    ) -> crate::sync::Request<'_, endpoints::OrgCheckQuota<'_>, bool> {
        endpoints::OrgCheckQuota { org, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the packages affecting the organization's quota
    ///
    /// - `org`: name of the organization
    pub fn org_list_quota_packages(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListQuotaPackages<'_>,
        (QuotaUsedPackageListHeaders, Vec<QuotaUsedPackage>),
    > {
        endpoints::OrgListQuotaPackages { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Rename an organization
    ///
    /// - `org`: existing org name
    /// - `body`: See [`RenameOrgOption`]
    pub fn rename_org(
        &self,
        org: &str,
        body: RenameOrgOption,
    ) -> crate::sync::Request<'_, endpoints::RenameOrg<'_>, ()> {
        endpoints::RenameOrg { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List an organization's repos
    ///
    /// - `org`: name of the organization
    pub fn org_list_repos(
        &self,
        org: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListRepos<'_>,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::OrgListRepos { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a repository in an organization
    ///
    /// - `org`: name of organization
    /// - `body`: See [`CreateRepoOption`]
    pub fn create_org_repo(
        &self,
        org: &str,
        body: CreateRepoOption,
    ) -> crate::sync::Request<'_, endpoints::CreateOrgRepo<'_>, Repository> {
        endpoints::CreateOrgRepo { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List an organization's teams
    ///
    /// - `org`: name of the organization
    pub fn org_list_teams(
        &self,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListTeams<'_>, (TeamListHeaders, Vec<Team>)> {
        endpoints::OrgListTeams { org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a team
    ///
    /// - `org`: name of the organization
    /// - `body`: See [`CreateTeamOption`]
    pub fn org_create_team(
        &self,
        org: &str,
        body: CreateTeamOption,
    ) -> crate::sync::Request<'_, endpoints::OrgCreateTeam<'_>, Team> {
        endpoints::OrgCreateTeam { org, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search for teams within an organization
    ///
    /// - `org`: name of the organization
    pub fn team_search(
        &self,
        org: &str,
        query: TeamSearchQuery,
    ) -> crate::sync::Request<'_, endpoints::TeamSearch<'_>, TeamSearchResults> {
        endpoints::TeamSearch { org, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Unblock a user from the organization
    ///
    /// - `org`: name of the org
    /// - `username`: username of the user
    pub fn org_unblock_user(
        &self,
        org: &str,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgUnblockUser<'_>, ()> {
        endpoints::OrgUnblockUser { org, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Gets all packages of an owner
    ///
    /// - `owner`: owner of the packages
    pub fn list_packages(
        &self,
        owner: &str,
        query: ListPackagesQuery,
    ) -> crate::sync::Request<'_, endpoints::ListPackages<'_>, (PackageListHeaders, Vec<Package>)>
    {
        endpoints::ListPackages { owner, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Link a package to a repository
    ///
    /// - `owner`: owner of the package
    /// - `type`: type of the package
    /// - `name`: name of the package
    /// - `repo_name`: name of the repository to link.
    pub fn link_package(
        &self,
        owner: &str,
        r#type: &str,
        name: &str,
        repo_name: &str,
    ) -> crate::sync::Request<'_, endpoints::LinkPackage<'_>, ()> {
        endpoints::LinkPackage {
            owner,
            r#type,
            name,
            repo_name,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Unlink a package from a repository
    ///
    /// - `owner`: owner of the package
    /// - `type`: type of the package
    /// - `name`: name of the package
    pub fn unlink_package(
        &self,
        owner: &str,
        r#type: &str,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::UnlinkPackage<'_>, ()> {
        endpoints::UnlinkPackage {
            owner,
            r#type,
            name,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Gets a package
    ///
    /// - `owner`: owner of the package
    /// - `type`: type of the package
    /// - `name`: name of the package
    /// - `version`: version of the package
    pub fn get_package(
        &self,
        owner: &str,
        r#type: &str,
        name: &str,
        version: &str,
    ) -> crate::sync::Request<'_, endpoints::GetPackage<'_>, Package> {
        endpoints::GetPackage {
            owner,
            r#type,
            name,
            version,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a package
    ///
    /// - `owner`: owner of the package
    /// - `type`: type of the package
    /// - `name`: name of the package
    /// - `version`: version of the package
    pub fn delete_package(
        &self,
        owner: &str,
        r#type: &str,
        name: &str,
        version: &str,
    ) -> crate::sync::Request<'_, endpoints::DeletePackage<'_>, ()> {
        endpoints::DeletePackage {
            owner,
            r#type,
            name,
            version,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Gets all files of a package
    ///
    /// - `owner`: owner of the package
    /// - `type`: type of the package
    /// - `name`: name of the package
    /// - `version`: version of the package
    pub fn list_package_files(
        &self,
        owner: &str,
        r#type: &str,
        name: &str,
        version: &str,
    ) -> crate::sync::Request<'_, endpoints::ListPackageFiles<'_>, Vec<PackageFile>> {
        endpoints::ListPackageFiles {
            owner,
            r#type,
            name,
            version,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Search for issues across the repositories that the user has access to
    ///
    pub fn issue_search_issues(
        &self,
        query: IssueSearchIssuesQuery,
    ) -> crate::sync::Request<'_, endpoints::IssueSearchIssues, (IssueListHeaders, Vec<Issue>)>
    {
        endpoints::IssueSearchIssues { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Migrate a remote git repository
    ///
    /// - `body`: See [`MigrateRepoOptions`]
    pub fn repo_migrate(
        &self,
        body: MigrateRepoOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoMigrate, Repository> {
        endpoints::RepoMigrate { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search for repositories
    ///
    pub fn repo_search(
        &self,
        query: RepoSearchQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoSearch, SearchResults> {
        endpoints::RepoSearch { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGet<'_>, Repository> {
        endpoints::RepoGet { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a repository
    ///
    /// - `owner`: owner of the repo to delete
    /// - `repo`: name of the repo to delete
    pub fn repo_delete(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDelete<'_>, ()> {
        endpoints::RepoDelete { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a repository's properties. Only fields that are set will be changed.
    ///
    /// - `owner`: owner of the repo to edit
    /// - `repo`: name of the repo to edit
    /// - `body`: Properties of a repo that you can edit

    ///   See [`EditRepoOption`]
    pub fn repo_edit(
        &self,
        owner: &str,
        repo: &str,
        body: EditRepoOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEdit<'_>, Repository> {
        endpoints::RepoEdit {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Search for repository's action jobs according filter conditions
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_search_run_jobs(
        &self,
        owner: &str,
        repo: &str,
        query: RepoSearchRunJobsQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoSearchRunJobs<'_>, Vec<ActionRunJob>> {
        endpoints::RepoSearchRunJobs { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a repository's actions runner registration token
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_runner_registration_token(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetRunnerRegistrationToken<'_>, RegistrationToken>
    {
        endpoints::RepoGetRunnerRegistrationToken { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repository's action runs
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn list_action_runs(
        &self,
        owner: &str,
        repo: &str,
        query: ListActionRunsQuery,
    ) -> crate::sync::Request<'_, endpoints::ListActionRuns<'_>, ListActionRunResponse> {
        endpoints::ListActionRuns { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an action run
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `run_id`: id of the action run
    pub fn get_action_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: i64,
    ) -> crate::sync::Request<'_, endpoints::GetActionRun<'_>, ActionRun> {
        endpoints::GetActionRun {
            owner,
            repo,
            run_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List an repo's actions secrets
    ///
    /// - `owner`: owner of the repository
    /// - `repo`: name of the repository
    pub fn repo_list_actions_secrets(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListActionsSecrets<'_>,
        (SecretListHeaders, Vec<Secret>),
    > {
        endpoints::RepoListActionsSecrets { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create or Update a secret value in a repository
    ///
    /// - `owner`: owner of the repository
    /// - `repo`: name of the repository
    /// - `secretname`: name of the secret
    /// - `body`: See [`CreateOrUpdateSecretOption`]
    pub fn update_repo_secret(
        &self,
        owner: &str,
        repo: &str,
        secretname: &str,
        body: CreateOrUpdateSecretOption,
    ) -> crate::sync::Request<'_, endpoints::UpdateRepoSecret<'_>, ()> {
        endpoints::UpdateRepoSecret {
            owner,
            repo,
            secretname,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a secret in a repository
    ///
    /// - `owner`: owner of the repository
    /// - `repo`: name of the repository
    /// - `secretname`: name of the secret
    pub fn delete_repo_secret(
        &self,
        owner: &str,
        repo: &str,
        secretname: &str,
    ) -> crate::sync::Request<'_, endpoints::DeleteRepoSecret<'_>, ()> {
        endpoints::DeleteRepoSecret {
            owner,
            repo,
            secretname,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's action tasks
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn list_action_tasks(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::ListActionTasks<'_>, ActionTaskResponse> {
        endpoints::ListActionTasks { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get repo-level variables list
    ///
    /// - `owner`: name of the owner
    /// - `repo`: name of the repository
    pub fn get_repo_variables_list(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::GetRepoVariablesList<'_>,
        (VariableListHeaders, Vec<ActionVariable>),
    > {
        endpoints::GetRepoVariablesList { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a repo-level variable
    ///
    /// - `owner`: name of the owner
    /// - `repo`: name of the repository
    /// - `variablename`: name of the variable
    pub fn get_repo_variable(
        &self,
        owner: &str,
        repo: &str,
        variablename: &str,
    ) -> crate::sync::Request<'_, endpoints::GetRepoVariable<'_>, ActionVariable> {
        endpoints::GetRepoVariable {
            owner,
            repo,
            variablename,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Update a repo-level variable
    ///
    /// - `owner`: name of the owner
    /// - `repo`: name of the repository
    /// - `variablename`: name of the variable
    /// - `body`: See [`UpdateVariableOption`]
    pub fn update_repo_variable(
        &self,
        owner: &str,
        repo: &str,
        variablename: &str,
        body: UpdateVariableOption,
    ) -> crate::sync::Request<'_, endpoints::UpdateRepoVariable<'_>, ()> {
        endpoints::UpdateRepoVariable {
            owner,
            repo,
            variablename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a repo-level variable
    ///
    /// - `owner`: name of the owner
    /// - `repo`: name of the repository
    /// - `variablename`: name of the variable
    /// - `body`: See [`CreateVariableOption`]
    pub fn create_repo_variable(
        &self,
        owner: &str,
        repo: &str,
        variablename: &str,
        body: CreateVariableOption,
    ) -> crate::sync::Request<'_, endpoints::CreateRepoVariable<'_>, ()> {
        endpoints::CreateRepoVariable {
            owner,
            repo,
            variablename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a repo-level variable
    ///
    /// - `owner`: name of the owner
    /// - `repo`: name of the repository
    /// - `variablename`: name of the variable
    pub fn delete_repo_variable(
        &self,
        owner: &str,
        repo: &str,
        variablename: &str,
    ) -> crate::sync::Request<'_, endpoints::DeleteRepoVariable<'_>, ()> {
        endpoints::DeleteRepoVariable {
            owner,
            repo,
            variablename,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Dispatches a workflow
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `workflowfilename`: name of the workflow
    /// - `body`: See [`DispatchWorkflowOption`]
    pub fn dispatch_workflow(
        &self,
        owner: &str,
        repo: &str,
        workflowfilename: &str,
        body: DispatchWorkflowOption,
    ) -> crate::sync::Request<'_, endpoints::DispatchWorkflow<'_>, Option<DispatchWorkflowRun>>
    {
        endpoints::DispatchWorkflow {
            owner,
            repo,
            workflowfilename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's activity feeds
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_activity_feeds(
        &self,
        owner: &str,
        repo: &str,
        query: RepoListActivityFeedsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListActivityFeeds<'_>,
        (ActivityFeedsListHeaders, Vec<Activity>),
    > {
        endpoints::RepoListActivityFeeds { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an archive of a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `archive`: the git reference for download with attached archive format (e.g. master.zip)
    pub fn repo_get_archive(
        &self,
        owner: &str,
        repo: &str,
        archive: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetArchive<'_>, Bytes> {
        endpoints::RepoGetArchive {
            owner,
            repo,
            archive,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Return all users that have write access and can be assigned to issues
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_assignees(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetAssignees<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::RepoGetAssignees { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a repository's avatar
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`UpdateRepoAvatarOption`]
    pub fn repo_update_avatar(
        &self,
        owner: &str,
        repo: &str,
        body: UpdateRepoAvatarOption,
    ) -> crate::sync::Request<'_, endpoints::RepoUpdateAvatar<'_>, ()> {
        endpoints::RepoUpdateAvatar {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a repository's avatar
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_delete_avatar(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteAvatar<'_>, ()> {
        endpoints::RepoDeleteAvatar { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List branch protections for a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_branch_protection(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListBranchProtection<'_>, Vec<BranchProtection>>
    {
        endpoints::RepoListBranchProtection { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a branch protections for a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateBranchProtectionOption`]
    pub fn repo_create_branch_protection(
        &self,
        owner: &str,
        repo: &str,
        body: CreateBranchProtectionOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateBranchProtection<'_>, BranchProtection> {
        endpoints::RepoCreateBranchProtection {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a specific branch protection for the repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `name`: name of protected branch
    pub fn repo_get_branch_protection(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetBranchProtection<'_>, BranchProtection> {
        endpoints::RepoGetBranchProtection { owner, repo, name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a specific branch protection for the repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `name`: name of protected branch
    pub fn repo_delete_branch_protection(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteBranchProtection<'_>, ()> {
        endpoints::RepoDeleteBranchProtection { owner, repo, name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a branch protections for a repository. Only fields that are set will be changed
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `name`: name of protected branch
    /// - `body`: See [`EditBranchProtectionOption`]
    pub fn repo_edit_branch_protection(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
        body: EditBranchProtectionOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEditBranchProtection<'_>, BranchProtection> {
        endpoints::RepoEditBranchProtection {
            owner,
            repo,
            name,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's branches
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_branches(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListBranches<'_>, (BranchListHeaders, Vec<Branch>)>
    {
        endpoints::RepoListBranches { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a branch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateBranchRepoOption`]
    pub fn repo_create_branch(
        &self,
        owner: &str,
        repo: &str,
        body: CreateBranchRepoOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateBranch<'_>, Branch> {
        endpoints::RepoCreateBranch {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Retrieve a specific branch from a repository, including its effective branch protection
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `branch`: branch to get
    pub fn repo_get_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetBranch<'_>, Branch> {
        endpoints::RepoGetBranch {
            owner,
            repo,
            branch,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a specific branch from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `branch`: branch to delete
    pub fn repo_delete_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteBranch<'_>, ()> {
        endpoints::RepoDeleteBranch {
            owner,
            repo,
            branch,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Update a branch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `branch`: name of the branch
    /// - `body`: See [`UpdateBranchRepoOption`]
    pub fn repo_update_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        body: UpdateBranchRepoOption,
    ) -> crate::sync::Request<'_, endpoints::RepoUpdateBranch<'_>, ()> {
        endpoints::RepoUpdateBranch {
            owner,
            repo,
            branch,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's collaborators
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_collaborators(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListCollaborators<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::RepoListCollaborators { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if a user is a collaborator of a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `collaborator`: username of the collaborator
    pub fn repo_check_collaborator(
        &self,
        owner: &str,
        repo: &str,
        collaborator: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoCheckCollaborator<'_>, ()> {
        endpoints::RepoCheckCollaborator {
            owner,
            repo,
            collaborator,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add a collaborator to a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `collaborator`: username of the collaborator to add
    /// - `body`: See [`AddCollaboratorOption`]
    pub fn repo_add_collaborator(
        &self,
        owner: &str,
        repo: &str,
        collaborator: &str,
        body: AddCollaboratorOption,
    ) -> crate::sync::Request<'_, endpoints::RepoAddCollaborator<'_>, ()> {
        endpoints::RepoAddCollaborator {
            owner,
            repo,
            collaborator,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a collaborator from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `collaborator`: username of the collaborator to delete
    pub fn repo_delete_collaborator(
        &self,
        owner: &str,
        repo: &str,
        collaborator: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteCollaborator<'_>, ()> {
        endpoints::RepoDeleteCollaborator {
            owner,
            repo,
            collaborator,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get repository permissions for a user
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `collaborator`: username of the collaborator
    pub fn repo_get_repo_permissions(
        &self,
        owner: &str,
        repo: &str,
        collaborator: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetRepoPermissions<'_>, RepoCollaboratorPermission>
    {
        endpoints::RepoGetRepoPermissions {
            owner,
            repo,
            collaborator,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a list of all commits from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_all_commits(
        &self,
        owner: &str,
        repo: &str,
        query: RepoGetAllCommitsQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetAllCommits<'_>, (CommitListHeaders, Vec<Commit>)>
    {
        endpoints::RepoGetAllCommits { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a commit's combined status, by branch/tag/commit reference
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `ref`: name of branch/tag/commit
    pub fn repo_get_combined_status_by_ref(
        &self,
        owner: &str,
        repo: &str,
        r#ref: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoGetCombinedStatusByRef<'_>,
        (CombinedStatusHeaders, CombinedStatus),
    > {
        endpoints::RepoGetCombinedStatusByRef { owner, repo, r#ref }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a commit's statuses, by branch/tag/commit reference
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `ref`: name of branch/tag/commit
    pub fn repo_list_statuses_by_ref(
        &self,
        owner: &str,
        repo: &str,
        r#ref: &str,
        query: RepoListStatusesByRefQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListStatusesByRef<'_>,
        (CommitStatusListHeaders, Vec<CommitStatus>),
    > {
        endpoints::RepoListStatusesByRef {
            owner,
            repo,
            r#ref,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get the pull request of the commit
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: SHA of the commit to get
    pub fn repo_get_commit_pull_request(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetCommitPullRequest<'_>, PullRequest> {
        endpoints::RepoGetCommitPullRequest { owner, repo, sha }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get commit comparison information
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `basehead`: compare two branches or commits
    pub fn repo_compare_diff(
        &self,
        owner: &str,
        repo: &str,
        basehead: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoCompareDiff<'_>, Compare> {
        endpoints::RepoCompareDiff {
            owner,
            repo,
            basehead,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Gets the metadata of all the entries of the root dir
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_contents_list(
        &self,
        owner: &str,
        repo: &str,
        query: RepoGetContentsListQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetContentsList<'_>, Vec<ContentsResponse>> {
        endpoints::RepoGetContentsList { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Modify multiple files in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`ChangeFilesOptions`]
    pub fn repo_change_files(
        &self,
        owner: &str,
        repo: &str,
        body: ChangeFilesOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoChangeFiles<'_>, FilesResponse> {
        endpoints::RepoChangeFiles {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Gets the metadata and contents (if a file) of an entry in a repository, or a list of entries if a dir
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: path of the dir, file, symlink or submodule in the repo
    pub fn repo_get_contents(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        query: RepoGetContentsQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetContents<'_>, ContentsResponse> {
        endpoints::RepoGetContents {
            owner,
            repo,
            filepath,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Update a file in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: path of the file to update
    /// - `body`: See [`UpdateFileOptions`]
    pub fn repo_update_file(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        body: UpdateFileOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoUpdateFile<'_>, FileResponse> {
        endpoints::RepoUpdateFile {
            owner,
            repo,
            filepath,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a file in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: path of the file to create
    /// - `body`: See [`CreateFileOptions`]
    pub fn repo_create_file(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        body: CreateFileOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateFile<'_>, FileResponse> {
        endpoints::RepoCreateFile {
            owner,
            repo,
            filepath,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a file in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: path of the file to delete
    /// - `body`: See [`DeleteFileOptions`]
    pub fn repo_delete_file(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        body: DeleteFileOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteFile<'_>, FileDeleteResponse> {
        endpoints::RepoDeleteFile {
            owner,
            repo,
            filepath,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Convert a mirror repo to a normal repo.
    ///
    /// - `owner`: owner of the repo to convert
    /// - `repo`: name of the repo to convert
    pub fn repo_convert(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoConvert<'_>, Repository> {
        endpoints::RepoConvert { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Apply diff patch to repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`UpdateFileOptions`]
    pub fn repo_apply_diff_patch(
        &self,
        owner: &str,
        repo: &str,
        body: UpdateFileOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoApplyDiffPatch<'_>, FileResponse> {
        endpoints::RepoApplyDiffPatch {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get the EditorConfig definitions of a file in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: filepath of file to get
    pub fn repo_get_editor_config(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        query: RepoGetEditorConfigQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetEditorConfig<'_>, BTreeMap<String, String>>
    {
        endpoints::RepoGetEditorConfig {
            owner,
            repo,
            filepath,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's flags
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_flags(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListFlags<'_>, Vec<String>> {
        endpoints::RepoListFlags { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Replace all flags of a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`ReplaceFlagsOption`]
    pub fn repo_replace_all_flags(
        &self,
        owner: &str,
        repo: &str,
        body: ReplaceFlagsOption,
    ) -> crate::sync::Request<'_, endpoints::RepoReplaceAllFlags<'_>, ()> {
        endpoints::RepoReplaceAllFlags {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove all flags from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_delete_all_flags(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteAllFlags<'_>, ()> {
        endpoints::RepoDeleteAllFlags { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if a repository has a given flag
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `flag`: name of the flag
    pub fn repo_check_flag(
        &self,
        owner: &str,
        repo: &str,
        flag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoCheckFlag<'_>, ()> {
        endpoints::RepoCheckFlag { owner, repo, flag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a flag to a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `flag`: name of the flag
    pub fn repo_add_flag(
        &self,
        owner: &str,
        repo: &str,
        flag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoAddFlag<'_>, ()> {
        endpoints::RepoAddFlag { owner, repo, flag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Remove a flag from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `flag`: name of the flag
    pub fn repo_delete_flag(
        &self,
        owner: &str,
        repo: &str,
        flag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteFlag<'_>, ()> {
        endpoints::RepoDeleteFlag { owner, repo, flag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repository's forks
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn list_forks(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::ListForks<'_>, (RepositoryListHeaders, Vec<Repository>)>
    {
        endpoints::ListForks { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Fork a repository
    ///
    /// - `owner`: owner of the repo to fork
    /// - `repo`: name of the repo to fork
    /// - `body`: See [`CreateForkOption`]
    pub fn create_fork(
        &self,
        owner: &str,
        repo: &str,
        body: CreateForkOption,
    ) -> crate::sync::Request<'_, endpoints::CreateFork<'_>, Repository> {
        endpoints::CreateFork {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Gets multiple blobs of a repository.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn get_blobs(
        &self,
        owner: &str,
        repo: &str,
        query: GetBlobsQuery,
    ) -> crate::sync::Request<'_, endpoints::GetBlobs<'_>, Vec<GitBlob>> {
        endpoints::GetBlobs { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Gets the blob of a repository.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: sha of the blob to retrieve
    pub fn get_blob(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> crate::sync::Request<'_, endpoints::GetBlob<'_>, GitBlob> {
        endpoints::GetBlob { owner, repo, sha }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a single commit from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: a git ref or commit sha
    pub fn repo_get_single_commit(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        query: RepoGetSingleCommitQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetSingleCommit<'_>, Commit> {
        endpoints::RepoGetSingleCommit {
            owner,
            repo,
            sha,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a commit's diff or patch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: SHA of the commit to get
    /// - `diffType`: whether the output is diff or patch
    pub fn repo_download_commit_diff_or_patch(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        diff_type: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDownloadCommitDiffOrPatch<'_>, String> {
        endpoints::RepoDownloadCommitDiffOrPatch {
            owner,
            repo,
            sha,
            diff_type,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a note corresponding to a single commit from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: a git ref or commit sha
    pub fn repo_get_note(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        query: RepoGetNoteQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetNote<'_>, Note> {
        endpoints::RepoGetNote {
            owner,
            repo,
            sha,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Set a note corresponding to a single commit from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: a git ref or commit sha
    /// - `body`: See [`NoteOptions`]
    pub fn repo_set_note(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        body: NoteOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoSetNote<'_>, Note> {
        endpoints::RepoSetNote {
            owner,
            repo,
            sha,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Removes a note corresponding to a single commit from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: a git ref or commit sha
    pub fn repo_remove_note(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoRemoveNote<'_>, ()> {
        endpoints::RepoRemoveNote { owner, repo, sha }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get specified ref or filtered repository's refs
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_all_git_refs(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListAllGitRefs<'_>, Vec<Reference>> {
        endpoints::RepoListAllGitRefs { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get specified ref or filtered repository's refs
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `ref`: part or full name of the ref
    pub fn repo_list_git_refs(
        &self,
        owner: &str,
        repo: &str,
        r#ref: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListGitRefs<'_>, Vec<Reference>> {
        endpoints::RepoListGitRefs { owner, repo, r#ref }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Gets the tag object of an annotated tag (not lightweight tags)
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: sha of the tag. The Git tags API only supports annotated tag objects, not lightweight tags.
    pub fn get_annotated_tag(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
    ) -> crate::sync::Request<'_, endpoints::GetAnnotatedTag<'_>, AnnotatedTag> {
        endpoints::GetAnnotatedTag { owner, repo, sha }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Gets the tree of a repository.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: sha of the commit
    pub fn get_tree(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        query: GetTreeQuery,
    ) -> crate::sync::Request<'_, endpoints::GetTree<'_>, GitTreeResponse> {
        endpoints::GetTree {
            owner,
            repo,
            sha,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List the hooks in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_hooks(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListHooks<'_>, (HookListHeaders, Vec<Hook>)> {
        endpoints::RepoListHooks { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a hook
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateHookOption`]
    pub fn repo_create_hook(
        &self,
        owner: &str,
        repo: &str,
        body: CreateHookOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateHook<'_>, Hook> {
        endpoints::RepoCreateHook {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List the Git hooks in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_git_hooks(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListGitHooks<'_>, Vec<GitHook>> {
        endpoints::RepoListGitHooks { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a Git hook
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the hook to get
    pub fn repo_get_git_hook(
        &self,
        owner: &str,
        repo: &str,
        id: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetGitHook<'_>, GitHook> {
        endpoints::RepoGetGitHook { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a Git hook in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the hook to get
    pub fn repo_delete_git_hook(
        &self,
        owner: &str,
        repo: &str,
        id: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteGitHook<'_>, ()> {
        endpoints::RepoDeleteGitHook { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a Git hook in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the hook to get
    /// - `body`: See [`EditGitHookOption`]
    pub fn repo_edit_git_hook(
        &self,
        owner: &str,
        repo: &str,
        id: &str,
        body: EditGitHookOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEditGitHook<'_>, GitHook> {
        endpoints::RepoEditGitHook {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a hook
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the hook to get
    pub fn repo_get_hook(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetHook<'_>, Hook> {
        endpoints::RepoGetHook { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a hook in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the hook to delete
    pub fn repo_delete_hook(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteHook<'_>, ()> {
        endpoints::RepoDeleteHook { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a hook in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: index of the hook
    /// - `body`: See [`EditHookOption`]
    pub fn repo_edit_hook(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        body: EditHookOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEditHook<'_>, Hook> {
        endpoints::RepoEditHook {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Test a push webhook
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the hook to test
    pub fn repo_test_hook(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        query: RepoTestHookQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoTestHook<'_>, ()> {
        endpoints::RepoTestHook {
            owner,
            repo,
            id,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Returns the issue config for a repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_issue_config(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetIssueConfig<'_>, IssueConfig> {
        endpoints::RepoGetIssueConfig { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns the validation information for a issue config
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_validate_issue_config(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoValidateIssueConfig<'_>, IssueConfigValidation>
    {
        endpoints::RepoValidateIssueConfig { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get available issue templates for a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_issue_templates(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetIssueTemplates<'_>, Vec<IssueTemplate>> {
        endpoints::RepoGetIssueTemplates { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repository's issues
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn issue_list_issues(
        &self,
        owner: &str,
        repo: &str,
        query: IssueListIssuesQuery,
    ) -> crate::sync::Request<'_, endpoints::IssueListIssues<'_>, (IssueListHeaders, Vec<Issue>)>
    {
        endpoints::IssueListIssues { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create an issue. If using deadline only the date will be taken into account, and time of day ignored.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateIssueOption`]
    pub fn issue_create_issue(
        &self,
        owner: &str,
        repo: &str,
        body: CreateIssueOption,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateIssue<'_>, Issue> {
        endpoints::IssueCreateIssue {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List all comments in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn issue_get_repo_comments(
        &self,
        owner: &str,
        repo: &str,
        query: IssueGetRepoCommentsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::IssueGetRepoComments<'_>,
        (CommentListHeaders, Vec<Comment>),
    > {
        endpoints::IssueGetRepoComments { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment
    pub fn issue_get_comment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetComment<'_>, Option<Comment>> {
        endpoints::IssueGetComment { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of comment to delete
    pub fn issue_delete_comment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteComment<'_>, ()> {
        endpoints::IssueDeleteComment { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment to edit
    /// - `body`: See [`EditIssueCommentOption`]
    pub fn issue_edit_comment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        body: EditIssueCommentOption,
    ) -> crate::sync::Request<'_, endpoints::IssueEditComment<'_>, Option<Comment>> {
        endpoints::IssueEditComment {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List comment's attachments
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment
    pub fn issue_list_issue_comment_attachments(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueListIssueCommentAttachments<'_>, Vec<Attachment>>
    {
        endpoints::IssueListIssueCommentAttachments { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a comment attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment
    /// - `attachment`: attachment to upload
    pub fn issue_create_issue_comment_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment: &[u8],
        query: IssueCreateIssueCommentAttachmentQuery,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateIssueCommentAttachment<'_>, Attachment>
    {
        endpoints::IssueCreateIssueCommentAttachment {
            owner,
            repo,
            id,
            attachment: &attachment,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a comment attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment
    /// - `attachment_id`: id of the attachment to get
    pub fn issue_get_issue_comment_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment_id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetIssueCommentAttachment<'_>, Attachment> {
        endpoints::IssueGetIssueCommentAttachment {
            owner,
            repo,
            id,
            attachment_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a comment attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment
    /// - `attachment_id`: id of the attachment to delete
    pub fn issue_delete_issue_comment_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment_id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteIssueCommentAttachment<'_>, ()> {
        endpoints::IssueDeleteIssueCommentAttachment {
            owner,
            repo,
            id,
            attachment_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Edit a comment attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment
    /// - `attachment_id`: id of the attachment to edit
    /// - `body`: See [`EditAttachmentOptions`]
    pub fn issue_edit_issue_comment_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment_id: i64,
        body: EditAttachmentOptions,
    ) -> crate::sync::Request<'_, endpoints::IssueEditIssueCommentAttachment<'_>, Attachment> {
        endpoints::IssueEditIssueCommentAttachment {
            owner,
            repo,
            id,
            attachment_id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a list of reactions from a comment of an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment to edit
    pub fn issue_get_comment_reactions(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetCommentReactions<'_>, Vec<Reaction>> {
        endpoints::IssueGetCommentReactions { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a reaction to a comment of an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment to edit
    /// - `content`: See [`EditReactionOption`]
    pub fn issue_post_comment_reaction(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        content: EditReactionOption,
    ) -> crate::sync::Request<'_, endpoints::IssuePostCommentReaction<'_>, Reaction> {
        endpoints::IssuePostCommentReaction {
            owner,
            repo,
            id,
            body: content,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove a reaction from a comment of an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the comment to edit
    /// - `content`: See [`EditReactionOption`]
    pub fn issue_delete_comment_reaction(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        content: EditReactionOption,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteCommentReaction<'_>, ()> {
        endpoints::IssueDeleteCommentReaction {
            owner,
            repo,
            id,
            body: content,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repo's pinned issues
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_pinned_issues(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListPinnedIssues<'_>, Vec<Issue>> {
        endpoints::RepoListPinnedIssues { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to get
    pub fn issue_get_issue(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetIssue<'_>, Issue> {
        endpoints::IssueGetIssue { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of issue to delete
    pub fn issue_delete(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDelete<'_>, ()> {
        endpoints::IssueDelete { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit an issue. If using deadline only the date will be taken into account, and time of day ignored.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to edit
    /// - `body`: See [`EditIssueOption`]
    pub fn issue_edit_issue(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: EditIssueOption,
    ) -> crate::sync::Request<'_, endpoints::IssueEditIssue<'_>, Issue> {
        endpoints::IssueEditIssue {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List issue's attachments
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_list_issue_attachments(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueListIssueAttachments<'_>, Vec<Attachment>> {
        endpoints::IssueListIssueAttachments { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create an issue attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `attachment`: attachment to upload
    pub fn issue_create_issue_attachment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        attachment: &[u8],
        query: IssueCreateIssueAttachmentQuery,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateIssueAttachment<'_>, Attachment> {
        endpoints::IssueCreateIssueAttachment {
            owner,
            repo,
            index,
            attachment: &attachment,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get an issue attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `attachment_id`: id of the attachment to get
    pub fn issue_get_issue_attachment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        attachment_id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetIssueAttachment<'_>, Attachment> {
        endpoints::IssueGetIssueAttachment {
            owner,
            repo,
            index,
            attachment_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete an issue attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `attachment_id`: id of the attachment to delete
    pub fn issue_delete_issue_attachment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        attachment_id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteIssueAttachment<'_>, ()> {
        endpoints::IssueDeleteIssueAttachment {
            owner,
            repo,
            index,
            attachment_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Edit an issue attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `attachment_id`: id of the attachment to edit
    /// - `body`: See [`EditAttachmentOptions`]
    pub fn issue_edit_issue_attachment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        attachment_id: i64,
        body: EditAttachmentOptions,
    ) -> crate::sync::Request<'_, endpoints::IssueEditIssueAttachment<'_>, Attachment> {
        endpoints::IssueEditIssueAttachment {
            owner,
            repo,
            index,
            attachment_id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List issues that are blocked by this issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_list_blocks(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueListBlocks<'_>, Vec<Issue>> {
        endpoints::IssueListBlocks { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Block the issue given in the body by the issue in path
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`IssueMeta`]
    pub fn issue_create_issue_blocking(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: IssueMeta,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateIssueBlocking<'_>, Issue> {
        endpoints::IssueCreateIssueBlocking {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Unblock the issue given in the body by the issue in path
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`IssueMeta`]
    pub fn issue_remove_issue_blocking(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: IssueMeta,
    ) -> crate::sync::Request<'_, endpoints::IssueRemoveIssueBlocking<'_>, Issue> {
        endpoints::IssueRemoveIssueBlocking {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List all comments on an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_get_comments(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        query: IssueGetCommentsQuery,
    ) -> crate::sync::Request<'_, endpoints::IssueGetComments<'_>, (CommentListHeaders, Vec<Comment>)>
    {
        endpoints::IssueGetComments {
            owner,
            repo,
            index,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add a comment to an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`CreateIssueCommentOption`]
    pub fn issue_create_comment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: CreateIssueCommentOption,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateComment<'_>, Comment> {
        endpoints::IssueCreateComment {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: this parameter is ignored
    /// - `id`: id of comment to delete
    pub fn issue_delete_comment_deprecated(
        &self,
        owner: &str,
        repo: &str,
        index: u32,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteCommentDeprecated<'_>, ()> {
        endpoints::IssueDeleteCommentDeprecated {
            owner,
            repo,
            index,
            id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Edit a comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: this parameter is ignored
    /// - `id`: id of the comment to edit
    /// - `body`: See [`EditIssueCommentOption`]
    pub fn issue_edit_comment_deprecated(
        &self,
        owner: &str,
        repo: &str,
        index: u32,
        id: i64,
        body: EditIssueCommentOption,
    ) -> crate::sync::Request<'_, endpoints::IssueEditCommentDeprecated<'_>, Option<Comment>> {
        endpoints::IssueEditCommentDeprecated {
            owner,
            repo,
            index,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Set an issue deadline. If set to null, the deadline is deleted. If using deadline only the date will be taken into account, and time of day ignored.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to create or update a deadline on
    /// - `body`: See [`EditDeadlineOption`]
    pub fn issue_edit_issue_deadline(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: EditDeadlineOption,
    ) -> crate::sync::Request<'_, endpoints::IssueEditIssueDeadline<'_>, IssueDeadline> {
        endpoints::IssueEditIssueDeadline {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List an issue's dependencies, i.e all issues that block this issue.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_list_issue_dependencies(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueListIssueDependencies<'_>, Vec<Issue>> {
        endpoints::IssueListIssueDependencies { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Make the issue in the url depend on the issue in the form.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`IssueMeta`]
    pub fn issue_create_issue_dependencies(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: IssueMeta,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateIssueDependencies<'_>, Issue> {
        endpoints::IssueCreateIssueDependencies {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove an issue dependency
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`IssueMeta`]
    pub fn issue_remove_issue_dependencies(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: IssueMeta,
    ) -> crate::sync::Request<'_, endpoints::IssueRemoveIssueDependencies<'_>, Issue> {
        endpoints::IssueRemoveIssueDependencies {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get an issue's labels
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_get_labels(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetLabels<'_>, Vec<Label>> {
        endpoints::IssueGetLabels { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Replace an issue's labels
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`IssueLabelsOption`]
    pub fn issue_replace_labels(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: IssueLabelsOption,
    ) -> crate::sync::Request<'_, endpoints::IssueReplaceLabels<'_>, Vec<Label>> {
        endpoints::IssueReplaceLabels {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add a label to an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`IssueLabelsOption`]
    pub fn issue_add_label(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: IssueLabelsOption,
    ) -> crate::sync::Request<'_, endpoints::IssueAddLabel<'_>, Vec<Label>> {
        endpoints::IssueAddLabel {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove all labels from an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`DeleteLabelsOption`]
    pub fn issue_clear_labels(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: DeleteLabelsOption,
    ) -> crate::sync::Request<'_, endpoints::IssueClearLabels<'_>, ()> {
        endpoints::IssueClearLabels {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove a label from an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `identifier`: name or id of the label to remove
    /// - `body`: See [`DeleteLabelsOption`]
    pub fn issue_remove_label(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        identifier: &str,
        body: DeleteLabelsOption,
    ) -> crate::sync::Request<'_, endpoints::IssueRemoveLabel<'_>, ()> {
        endpoints::IssueRemoveLabel {
            owner,
            repo,
            index,
            identifier,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Pin an Issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of issue to pin
    pub fn pin_issue(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::PinIssue<'_>, ()> {
        endpoints::PinIssue { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Unpin an Issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of issue to unpin
    pub fn unpin_issue(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::UnpinIssue<'_>, ()> {
        endpoints::UnpinIssue { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Moves the Pin to the given Position
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of issue
    /// - `position`: the new position
    pub fn move_issue_pin(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        position: i64,
    ) -> crate::sync::Request<'_, endpoints::MoveIssuePin<'_>, ()> {
        endpoints::MoveIssuePin {
            owner,
            repo,
            index,
            position,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a list reactions of an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_get_issue_reactions(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<
        '_,
        endpoints::IssueGetIssueReactions<'_>,
        (ReactionListHeaders, Vec<Reaction>),
    > {
        endpoints::IssueGetIssueReactions { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a reaction to an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `content`: See [`EditReactionOption`]
    pub fn issue_post_issue_reaction(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        content: EditReactionOption,
    ) -> crate::sync::Request<'_, endpoints::IssuePostIssueReaction<'_>, Reaction> {
        endpoints::IssuePostIssueReaction {
            owner,
            repo,
            index,
            body: content,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Remove a reaction from an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `content`: See [`EditReactionOption`]
    pub fn issue_delete_issue_reaction(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        content: EditReactionOption,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteIssueReaction<'_>, ()> {
        endpoints::IssueDeleteIssueReaction {
            owner,
            repo,
            index,
            body: content,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete an issue's existing stopwatch.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to stop the stopwatch on
    pub fn issue_delete_stop_watch(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteStopWatch<'_>, ()> {
        endpoints::IssueDeleteStopWatch { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Start stopwatch on an issue.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to create the stopwatch on
    pub fn issue_start_stop_watch(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueStartStopWatch<'_>, ()> {
        endpoints::IssueStartStopWatch { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Stop an issue's existing stopwatch.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to stop the stopwatch on
    pub fn issue_stop_stop_watch(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueStopStopWatch<'_>, ()> {
        endpoints::IssueStopStopWatch { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get users who subscribed on an issue.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_subscriptions(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueSubscriptions<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::IssueSubscriptions { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if user is subscribed to an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_check_subscription(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueCheckSubscription<'_>, WatchInfo> {
        endpoints::IssueCheckSubscription { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Subscribe user to issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `user`: user to subscribe
    pub fn issue_add_subscription(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        user: &str,
    ) -> crate::sync::Request<'_, endpoints::IssueAddSubscription<'_>, ()> {
        endpoints::IssueAddSubscription {
            owner,
            repo,
            index,
            user,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Unsubscribe user from issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `user`: user witch unsubscribe
    pub fn issue_delete_subscription(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        user: &str,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteSubscription<'_>, ()> {
        endpoints::IssueDeleteSubscription {
            owner,
            repo,
            index,
            user,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List all comments and events on an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_get_comments_and_timeline(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        query: IssueGetCommentsAndTimelineQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::IssueGetCommentsAndTimeline<'_>,
        (TimelineListHeaders, Vec<TimelineComment>),
    > {
        endpoints::IssueGetCommentsAndTimeline {
            owner,
            repo,
            index,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List an issue's tracked times
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    pub fn issue_tracked_times(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        query: IssueTrackedTimesQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::IssueTrackedTimes<'_>,
        (TrackedTimeListHeaders, Vec<TrackedTime>),
    > {
        endpoints::IssueTrackedTimes {
            owner,
            repo,
            index,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add tracked time to a issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `body`: See [`AddTimeOption`]
    pub fn issue_add_time(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: AddTimeOption,
    ) -> crate::sync::Request<'_, endpoints::IssueAddTime<'_>, TrackedTime> {
        endpoints::IssueAddTime {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Reset a tracked time of an issue
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue to add tracked time to
    pub fn issue_reset_time(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueResetTime<'_>, ()> {
        endpoints::IssueResetTime { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete specific tracked time
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the issue
    /// - `id`: id of time to delete
    pub fn issue_delete_time(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteTime<'_>, ()> {
        endpoints::IssueDeleteTime {
            owner,
            repo,
            index,
            id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's keys
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_keys(
        &self,
        owner: &str,
        repo: &str,
        query: RepoListKeysQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoListKeys<'_>, (DeployKeyListHeaders, Vec<DeployKey>)>
    {
        endpoints::RepoListKeys { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a key to a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateKeyOption`]
    pub fn repo_create_key(
        &self,
        owner: &str,
        repo: &str,
        body: CreateKeyOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateKey<'_>, DeployKey> {
        endpoints::RepoCreateKey {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a repository's key by id
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the key to get
    pub fn repo_get_key(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetKey<'_>, DeployKey> {
        endpoints::RepoGetKey { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a key from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the key to delete
    pub fn repo_delete_key(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteKey<'_>, ()> {
        endpoints::RepoDeleteKey { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get all of a repository's labels
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn issue_list_labels(
        &self,
        owner: &str,
        repo: &str,
        query: IssueListLabelsQuery,
    ) -> crate::sync::Request<'_, endpoints::IssueListLabels<'_>, (LabelListHeaders, Vec<Label>)>
    {
        endpoints::IssueListLabels { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a label
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateLabelOption`]
    pub fn issue_create_label(
        &self,
        owner: &str,
        repo: &str,
        body: CreateLabelOption,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateLabel<'_>, Label> {
        endpoints::IssueCreateLabel {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a single label
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the label to get
    pub fn issue_get_label(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetLabel<'_>, Label> {
        endpoints::IssueGetLabel { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a label
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the label to delete
    pub fn issue_delete_label(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteLabel<'_>, ()> {
        endpoints::IssueDeleteLabel { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a label
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the label to edit
    /// - `body`: See [`EditLabelOption`]
    pub fn issue_edit_label(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        body: EditLabelOption,
    ) -> crate::sync::Request<'_, endpoints::IssueEditLabel<'_>, Label> {
        endpoints::IssueEditLabel {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get languages and number of bytes of code written
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_languages(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetLanguages<'_>, BTreeMap<String, i64>> {
        endpoints::RepoGetLanguages { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a file or it's LFS object from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: filepath of the file to get
    pub fn repo_get_raw_file_or_lfs(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        query: RepoGetRawFileOrLfsQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetRawFileOrLfs<'_>, Bytes> {
        endpoints::RepoGetRawFileOrLfs {
            owner,
            repo,
            filepath,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get all of a repository's opened milestones
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn issue_get_milestones_list(
        &self,
        owner: &str,
        repo: &str,
        query: IssueGetMilestonesListQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::IssueGetMilestonesList<'_>,
        (MilestoneListHeaders, Vec<Milestone>),
    > {
        endpoints::IssueGetMilestonesList { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a milestone
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateMilestoneOption`]
    pub fn issue_create_milestone(
        &self,
        owner: &str,
        repo: &str,
        body: CreateMilestoneOption,
    ) -> crate::sync::Request<'_, endpoints::IssueCreateMilestone<'_>, Milestone> {
        endpoints::IssueCreateMilestone {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a milestone
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: the milestone to get, identified by ID and if not available by name
    pub fn issue_get_milestone(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueGetMilestone<'_>, Milestone> {
        endpoints::IssueGetMilestone { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a milestone
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: the milestone to delete, identified by ID and if not available by name
    pub fn issue_delete_milestone(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::IssueDeleteMilestone<'_>, ()> {
        endpoints::IssueDeleteMilestone { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a milestone
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: the milestone to edit, identified by ID and if not available by name
    /// - `body`: See [`EditMilestoneOption`]
    pub fn issue_edit_milestone(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        body: EditMilestoneOption,
    ) -> crate::sync::Request<'_, endpoints::IssueEditMilestone<'_>, Milestone> {
        endpoints::IssueEditMilestone {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Sync a mirrored repository
    ///
    /// - `owner`: owner of the repo to sync
    /// - `repo`: name of the repo to sync
    pub fn repo_mirror_sync(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoMirrorSync<'_>, ()> {
        endpoints::RepoMirrorSync { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns if new Issue Pins are allowed
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_new_pin_allowed(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoNewPinAllowed<'_>, NewIssuePinsAllowed> {
        endpoints::RepoNewPinAllowed { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List users's notification threads on a specific repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn notify_get_repo_list(
        &self,
        owner: &str,
        repo: &str,
        query: NotifyGetRepoListQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::NotifyGetRepoList<'_>,
        (NotificationThreadListHeaders, Vec<NotificationThread>),
    > {
        endpoints::NotifyGetRepoList { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Mark notification threads as read, pinned or unread on a specific repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn notify_read_repo_list(
        &self,
        owner: &str,
        repo: &str,
        query: NotifyReadRepoListQuery,
    ) -> crate::sync::Request<'_, endpoints::NotifyReadRepoList<'_>, Vec<NotificationThread>> {
        endpoints::NotifyReadRepoList { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repo's pull requests. If a pull request is selected but fails to be retrieved for any reason, it will be a null value in the list of results.
    ///
    /// - `owner`: Owner of the repo
    /// - `repo`: Name of the repo
    pub fn repo_list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        query: RepoListPullRequestsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListPullRequests<'_>,
        (PullRequestListHeaders, Vec<PullRequest>),
    > {
        endpoints::RepoListPullRequests { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreatePullRequestOption`]
    pub fn repo_create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        body: CreatePullRequestOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreatePullRequest<'_>, PullRequest> {
        endpoints::RepoCreatePullRequest {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repo's pinned pull requests
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_pinned_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListPinnedPullRequests<'_>,
        (PullRequestListHeaders, Vec<PullRequest>),
    > {
        endpoints::RepoListPinnedPullRequests { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a pull request by base and head
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `base`: base of the pull request to get
    /// - `head`: head of the pull request to get
    pub fn repo_get_pull_request_by_base_head(
        &self,
        owner: &str,
        repo: &str,
        base: &str,
        head: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetPullRequestByBaseHead<'_>, PullRequest> {
        endpoints::RepoGetPullRequestByBaseHead {
            owner,
            repo,
            base,
            head,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to get
    pub fn repo_get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetPullRequest<'_>, PullRequest> {
        endpoints::RepoGetPullRequest { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a pull request. If using deadline only the date will be taken into account, and time of day ignored.
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to edit
    /// - `body`: See [`EditPullRequestOption`]
    pub fn repo_edit_pull_request(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: EditPullRequestOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEditPullRequest<'_>, PullRequest> {
        endpoints::RepoEditPullRequest {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a pull request diff or patch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to get
    /// - `diffType`: whether the output is diff or patch
    pub fn repo_download_pull_diff_or_patch(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        diff_type: &str,
        query: RepoDownloadPullDiffOrPatchQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoDownloadPullDiffOrPatch<'_>, String> {
        endpoints::RepoDownloadPullDiffOrPatch {
            owner,
            repo,
            index,
            diff_type,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get commits for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to get
    pub fn repo_get_pull_request_commits(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        query: RepoGetPullRequestCommitsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoGetPullRequestCommits<'_>,
        (CommitListHeaders, Vec<Commit>),
    > {
        endpoints::RepoGetPullRequestCommits {
            owner,
            repo,
            index,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get changed files for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to get
    pub fn repo_get_pull_request_files(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        query: RepoGetPullRequestFilesQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoGetPullRequestFiles<'_>,
        (ChangedFileListWithPaginationHeaders, Vec<ChangedFile>),
    > {
        endpoints::RepoGetPullRequestFiles {
            owner,
            repo,
            index,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Check if a pull request has been merged
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    pub fn repo_pull_request_is_merged(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoPullRequestIsMerged<'_>, ()> {
        endpoints::RepoPullRequestIsMerged { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Merge a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to merge
    /// - `body`: See [`MergePullRequestOption`]
    pub fn repo_merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: MergePullRequestOption,
    ) -> crate::sync::Request<'_, endpoints::RepoMergePullRequest<'_>, ()> {
        endpoints::RepoMergePullRequest {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Cancel the scheduled auto merge for the given pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to merge
    pub fn repo_cancel_scheduled_auto_merge(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoCancelScheduledAutoMerge<'_>, ()> {
        endpoints::RepoCancelScheduledAutoMerge { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create review requests for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `body`: See [`PullReviewRequestOptions`]
    pub fn repo_create_pull_review_requests(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: PullReviewRequestOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoCreatePullReviewRequests<'_>, Vec<PullReview>>
    {
        endpoints::RepoCreatePullReviewRequests {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Cancel review requests for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `body`: See [`PullReviewRequestOptions`]
    pub fn repo_delete_pull_review_requests(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: PullReviewRequestOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoDeletePullReviewRequests<'_>, ()> {
        endpoints::RepoDeletePullReviewRequests {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List all reviews for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    pub fn repo_list_pull_reviews(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListPullReviews<'_>,
        (PullReviewListHeaders, Vec<PullReview>),
    > {
        endpoints::RepoListPullReviews { owner, repo, index }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a review to an pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `body`: See [`CreatePullReviewOptions`]
    pub fn repo_create_pull_review(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        body: CreatePullReviewOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoCreatePullReview<'_>, PullReview> {
        endpoints::RepoCreatePullReview {
            owner,
            repo,
            index,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a specific review for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    pub fn repo_get_pull_review(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetPullReview<'_>, PullReview> {
        endpoints::RepoGetPullReview {
            owner,
            repo,
            index,
            id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Submit a pending review to an pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    /// - `body`: See [`SubmitPullReviewOptions`]
    pub fn repo_submit_pull_review(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
        body: SubmitPullReviewOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoSubmitPullReview<'_>, PullReview> {
        endpoints::RepoSubmitPullReview {
            owner,
            repo,
            index,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a specific review from a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    pub fn repo_delete_pull_review(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeletePullReview<'_>, ()> {
        endpoints::RepoDeletePullReview {
            owner,
            repo,
            index,
            id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a specific review for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    pub fn repo_get_pull_review_comments(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetPullReviewComments<'_>, Vec<PullReviewComment>>
    {
        endpoints::RepoGetPullReviewComments {
            owner,
            repo,
            index,
            id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add a new comment to a pull request review
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    /// - `body`: See [`serde_json::Value`]
    pub fn repo_create_pull_review_comment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
        body: serde_json::Value,
    ) -> crate::sync::Request<'_, endpoints::RepoCreatePullReviewComment<'_>, PullReviewComment>
    {
        endpoints::RepoCreatePullReviewComment {
            owner,
            repo,
            index,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a pull review comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    /// - `comment`: id of the comment
    pub fn repo_get_pull_review_comment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
        comment: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetPullReviewComment<'_>, PullReviewComment> {
        endpoints::RepoGetPullReviewComment {
            owner,
            repo,
            index,
            id,
            comment,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a pull review comment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    /// - `comment`: id of the comment
    pub fn repo_delete_pull_review_comment(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
        comment: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeletePullReviewComment<'_>, ()> {
        endpoints::RepoDeletePullReviewComment {
            owner,
            repo,
            index,
            id,
            comment,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Dismiss a review for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    /// - `body`: See [`DismissPullReviewOptions`]
    pub fn repo_dismiss_pull_review(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
        body: DismissPullReviewOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoDismissPullReview<'_>, PullReview> {
        endpoints::RepoDismissPullReview {
            owner,
            repo,
            index,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Cancel to dismiss a review for a pull request
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request
    /// - `id`: id of the review
    pub fn repo_un_dismiss_pull_review(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoUnDismissPullReview<'_>, PullReview> {
        endpoints::RepoUnDismissPullReview {
            owner,
            repo,
            index,
            id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Merge PR's baseBranch into headBranch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `index`: index of the pull request to get
    pub fn repo_update_pull_request(
        &self,
        owner: &str,
        repo: &str,
        index: i64,
        query: RepoUpdatePullRequestQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoUpdatePullRequest<'_>, ()> {
        endpoints::RepoUpdatePullRequest {
            owner,
            repo,
            index,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get all push mirrors of the repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_push_mirrors(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListPushMirrors<'_>,
        (PushMirrorListHeaders, Vec<PushMirror>),
    > {
        endpoints::RepoListPushMirrors { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Set up a new push mirror in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreatePushMirrorOption`]
    pub fn repo_add_push_mirror(
        &self,
        owner: &str,
        repo: &str,
        body: CreatePushMirrorOption,
    ) -> crate::sync::Request<'_, endpoints::RepoAddPushMirror<'_>, PushMirror> {
        endpoints::RepoAddPushMirror {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Sync all push mirrored repository
    ///
    /// - `owner`: owner of the repo to sync
    /// - `repo`: name of the repo to sync
    pub fn repo_push_mirror_sync(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoPushMirrorSync<'_>, ()> {
        endpoints::RepoPushMirrorSync { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get push mirror of the repository by remoteName
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `name`: remote name of push mirror
    pub fn repo_get_push_mirror_by_remote_name(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetPushMirrorByRemoteName<'_>, PushMirror> {
        endpoints::RepoGetPushMirrorByRemoteName { owner, repo, name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Remove a push mirror from a repository by remoteName
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `name`: remote name of the pushMirror
    pub fn repo_delete_push_mirror(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeletePushMirror<'_>, ()> {
        endpoints::RepoDeletePushMirror { owner, repo, name }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a file from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `filepath`: filepath of the file to get
    pub fn repo_get_raw_file(
        &self,
        owner: &str,
        repo: &str,
        filepath: &str,
        query: RepoGetRawFileQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoGetRawFile<'_>, Bytes> {
        endpoints::RepoGetRawFile {
            owner,
            repo,
            filepath,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repo's releases
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_releases(
        &self,
        owner: &str,
        repo: &str,
        query: RepoListReleasesQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoListReleases<'_>, (ReleaseListHeaders, Vec<Release>)>
    {
        endpoints::RepoListReleases { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a release
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateReleaseOption`]
    pub fn repo_create_release(
        &self,
        owner: &str,
        repo: &str,
        body: CreateReleaseOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateRelease<'_>, Release> {
        endpoints::RepoCreateRelease {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Gets the most recent non-prerelease, non-draft release of a repository, sorted by created_at
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_latest_release(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetLatestRelease<'_>, Release> {
        endpoints::RepoGetLatestRelease { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a release by tag name
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `tag`: tag name of the release to get
    pub fn repo_get_release_by_tag(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetReleaseByTag<'_>, Release> {
        endpoints::RepoGetReleaseByTag { owner, repo, tag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a release by tag name
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `tag`: tag name of the release to delete
    pub fn repo_delete_release_by_tag(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteReleaseByTag<'_>, ()> {
        endpoints::RepoDeleteReleaseByTag { owner, repo, tag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a release
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release to get
    pub fn repo_get_release(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetRelease<'_>, Release> {
        endpoints::RepoGetRelease { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a release
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release to delete
    pub fn repo_delete_release(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteRelease<'_>, ()> {
        endpoints::RepoDeleteRelease { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a release
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release to edit
    /// - `body`: See [`EditReleaseOption`]
    pub fn repo_edit_release(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        body: EditReleaseOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEditRelease<'_>, Release> {
        endpoints::RepoEditRelease {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List release's attachments
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release
    pub fn repo_list_release_attachments(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoListReleaseAttachments<'_>, Vec<Attachment>> {
        endpoints::RepoListReleaseAttachments { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a release attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release
    /// - `attachment`: attachment to upload (this parameter is incompatible with `external_url`)
    /// - `external_url`: url to external asset (this parameter is incompatible with `attachment`)
    pub fn repo_create_release_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment: Option<&[u8]>,
        external_url: Option<&str>,
        query: RepoCreateReleaseAttachmentQuery,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateReleaseAttachment<'_>, Attachment> {
        endpoints::RepoCreateReleaseAttachment {
            owner,
            repo,
            id,
            attachment: attachment.as_deref(),
            external_url: external_url.as_deref(),
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a release attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release
    /// - `attachment_id`: id of the attachment to get
    pub fn repo_get_release_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment_id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetReleaseAttachment<'_>, Attachment> {
        endpoints::RepoGetReleaseAttachment {
            owner,
            repo,
            id,
            attachment_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a release attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release
    /// - `attachment_id`: id of the attachment to delete
    pub fn repo_delete_release_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment_id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteReleaseAttachment<'_>, ()> {
        endpoints::RepoDeleteReleaseAttachment {
            owner,
            repo,
            id,
            attachment_id,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Edit a release attachment
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the release
    /// - `attachment_id`: id of the attachment to edit
    /// - `body`: See [`EditAttachmentOptions`]
    pub fn repo_edit_release_attachment(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        attachment_id: i64,
        body: EditAttachmentOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoEditReleaseAttachment<'_>, Attachment> {
        endpoints::RepoEditReleaseAttachment {
            owner,
            repo,
            id,
            attachment_id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Return all users that can be requested to review in this repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_reviewers(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetReviewers<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::RepoGetReviewers { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get signing-key.gpg for given repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_signing_key(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoSigningKey<'_>, String> {
        endpoints::RepoSigningKey { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repo's stargazers
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_stargazers(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListStargazers<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::RepoListStargazers { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a commit's statuses
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: sha of the commit
    pub fn repo_list_statuses(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        query: RepoListStatusesQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoListStatuses<'_>,
        (CommitStatusListHeaders, Vec<CommitStatus>),
    > {
        endpoints::RepoListStatuses {
            owner,
            repo,
            sha,
            query,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a commit status
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `sha`: sha of the commit
    /// - `body`: See [`CreateStatusOption`]
    pub fn repo_create_status(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        body: CreateStatusOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateStatus<'_>, CommitStatus> {
        endpoints::RepoCreateStatus {
            owner,
            repo,
            sha,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repo's watchers
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_subscribers(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListSubscribers<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::RepoListSubscribers { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if the current user is watching a repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn user_current_check_subscription(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentCheckSubscription<'_>, WatchInfo> {
        endpoints::UserCurrentCheckSubscription { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Watch a repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn user_current_put_subscription(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentPutSubscription<'_>, WatchInfo> {
        endpoints::UserCurrentPutSubscription { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Unwatch a repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn user_current_delete_subscription(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentDeleteSubscription<'_>, ()> {
        endpoints::UserCurrentDeleteSubscription { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Gets information about syncing the fork default branch with the base branch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_sync_fork_default_info(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoSyncForkDefaultInfo<'_>, SyncForkInfo> {
        endpoints::RepoSyncForkDefaultInfo { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Syncs the default branch of a fork with the base branch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_sync_fork_default(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoSyncForkDefault<'_>, ()> {
        endpoints::RepoSyncForkDefault { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Gets information about syncing a fork branch with the base branch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `branch`: The branch
    pub fn repo_sync_fork_branch_info(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoSyncForkBranchInfo<'_>, SyncForkInfo> {
        endpoints::RepoSyncForkBranchInfo {
            owner,
            repo,
            branch,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Syncs a fork branch with the base branch
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `branch`: The branch
    pub fn repo_sync_fork_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoSyncForkBranch<'_>, ()> {
        endpoints::RepoSyncForkBranch {
            owner,
            repo,
            branch,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List tag protections for a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_tag_protection(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListTagProtection<'_>, Vec<TagProtection>> {
        endpoints::RepoListTagProtection { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a tag protections for a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateTagProtectionOption`]
    pub fn repo_create_tag_protection(
        &self,
        owner: &str,
        repo: &str,
        body: CreateTagProtectionOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateTagProtection<'_>, TagProtection> {
        endpoints::RepoCreateTagProtection {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a specific tag protection for the repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of the tag protect to get
    pub fn repo_get_tag_protection(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetTagProtection<'_>, TagProtection> {
        endpoints::RepoGetTagProtection { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a specific tag protection for the repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of protected tag
    pub fn repo_delete_tag_protection(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteTagProtection<'_>, ()> {
        endpoints::RepoDeleteTagProtection { owner, repo, id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a tag protections for a repository. Only fields that are set will be changed
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `id`: id of protected tag
    /// - `body`: See [`EditTagProtectionOption`]
    pub fn repo_edit_tag_protection(
        &self,
        owner: &str,
        repo: &str,
        id: i64,
        body: EditTagProtectionOption,
    ) -> crate::sync::Request<'_, endpoints::RepoEditTagProtection<'_>, TagProtection> {
        endpoints::RepoEditTagProtection {
            owner,
            repo,
            id,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// List a repository's tags
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_tags(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListTags<'_>, (TagListHeaders, Vec<Tag>)> {
        endpoints::RepoListTags { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a new git tag in a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateTagOption`]
    pub fn repo_create_tag(
        &self,
        owner: &str,
        repo: &str,
        body: CreateTagOption,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateTag<'_>, Tag> {
        endpoints::RepoCreateTag {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get the tag of a repository by tag name
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `tag`: name of tag
    pub fn repo_get_tag(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetTag<'_>, Tag> {
        endpoints::RepoGetTag { owner, repo, tag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a repository's tag by name
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `tag`: name of tag to delete
    pub fn repo_delete_tag(
        &self,
        owner: &str,
        repo: &str,
        tag: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteTag<'_>, ()> {
        endpoints::RepoDeleteTag { owner, repo, tag }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repository's teams
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_teams(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListTeams<'_>, Vec<Team>> {
        endpoints::RepoListTeams { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if a team is assigned to a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `team`: team name
    pub fn repo_check_team(
        &self,
        owner: &str,
        repo: &str,
        team: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoCheckTeam<'_>, Team> {
        endpoints::RepoCheckTeam { owner, repo, team }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a team to a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `team`: team name
    pub fn repo_add_team(
        &self,
        owner: &str,
        repo: &str,
        team: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoAddTeam<'_>, ()> {
        endpoints::RepoAddTeam { owner, repo, team }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a team from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `team`: team name
    pub fn repo_delete_team(
        &self,
        owner: &str,
        repo: &str,
        team: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteTeam<'_>, ()> {
        endpoints::RepoDeleteTeam { owner, repo, team }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a repo's tracked times
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_tracked_times(
        &self,
        owner: &str,
        repo: &str,
        query: RepoTrackedTimesQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoTrackedTimes<'_>,
        (TrackedTimeListHeaders, Vec<TrackedTime>),
    > {
        endpoints::RepoTrackedTimes { owner, repo, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a user's tracked times in a repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `user`: username of user
    pub fn user_tracked_times(
        &self,
        owner: &str,
        repo: &str,
        user: &str,
    ) -> crate::sync::Request<'_, endpoints::UserTrackedTimes<'_>, Vec<TrackedTime>> {
        endpoints::UserTrackedTimes { owner, repo, user }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get list of topics that a repository has
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_list_topics(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoListTopics<'_>, TopicName> {
        endpoints::RepoListTopics { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Replace list of topics for a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`RepoTopicOptions`]
    pub fn repo_update_topics(
        &self,
        owner: &str,
        repo: &str,
        body: RepoTopicOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoUpdateTopics<'_>, ()> {
        endpoints::RepoUpdateTopics {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Add a topic to a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `topic`: name of the topic to add
    pub fn repo_add_topic(
        &self,
        owner: &str,
        repo: &str,
        topic: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoAddTopic<'_>, ()> {
        endpoints::RepoAddTopic { owner, repo, topic }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a topic from a repository
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `topic`: name of the topic to delete
    pub fn repo_delete_topic(
        &self,
        owner: &str,
        repo: &str,
        topic: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteTopic<'_>, ()> {
        endpoints::RepoDeleteTopic { owner, repo, topic }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Transfer a repo ownership
    ///
    /// - `owner`: owner of the repo to transfer
    /// - `repo`: name of the repo to transfer
    /// - `body`: Transfer Options

    ///   See [`TransferRepoOption`]
    pub fn repo_transfer(
        &self,
        owner: &str,
        repo: &str,
        body: TransferRepoOption,
    ) -> crate::sync::Request<'_, endpoints::RepoTransfer<'_>, Repository> {
        endpoints::RepoTransfer {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Accept a repo transfer
    ///
    /// - `owner`: owner of the repo to transfer
    /// - `repo`: name of the repo to transfer
    pub fn accept_repo_transfer(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::AcceptRepoTransfer<'_>, Repository> {
        endpoints::AcceptRepoTransfer { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Reject a repo transfer
    ///
    /// - `owner`: owner of the repo to transfer
    /// - `repo`: name of the repo to transfer
    pub fn reject_repo_transfer(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::RejectRepoTransfer<'_>, Repository> {
        endpoints::RejectRepoTransfer { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a wiki page
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `body`: See [`CreateWikiPageOptions`]
    pub fn repo_create_wiki_page(
        &self,
        owner: &str,
        repo: &str,
        body: CreateWikiPageOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoCreateWikiPage<'_>, WikiPage> {
        endpoints::RepoCreateWikiPage {
            owner,
            repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a wiki page
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `pageName`: name of the page
    pub fn repo_get_wiki_page(
        &self,
        owner: &str,
        repo: &str,
        page_name: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoGetWikiPage<'_>, WikiPage> {
        endpoints::RepoGetWikiPage {
            owner,
            repo,
            page_name,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a wiki page
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `pageName`: name of the page
    pub fn repo_delete_wiki_page(
        &self,
        owner: &str,
        repo: &str,
        page_name: &str,
    ) -> crate::sync::Request<'_, endpoints::RepoDeleteWikiPage<'_>, ()> {
        endpoints::RepoDeleteWikiPage {
            owner,
            repo,
            page_name,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Edit a wiki page
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `pageName`: name of the page
    /// - `body`: See [`CreateWikiPageOptions`]
    pub fn repo_edit_wiki_page(
        &self,
        owner: &str,
        repo: &str,
        page_name: &str,
        body: CreateWikiPageOptions,
    ) -> crate::sync::Request<'_, endpoints::RepoEditWikiPage<'_>, WikiPage> {
        endpoints::RepoEditWikiPage {
            owner,
            repo,
            page_name,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get all wiki pages
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn repo_get_wiki_pages(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoGetWikiPages<'_>,
        (WikiPageListHeaders, Vec<WikiPageMetaData>),
    > {
        endpoints::RepoGetWikiPages { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get revisions of a wiki page
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    /// - `pageName`: name of the page
    pub fn repo_get_wiki_page_revisions(
        &self,
        owner: &str,
        repo: &str,
        page_name: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::RepoGetWikiPageRevisions<'_>,
        (WikiCommitListHeaders, WikiCommitList),
    > {
        endpoints::RepoGetWikiPageRevisions {
            owner,
            repo,
            page_name,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a repository using a template
    ///
    /// - `template_owner`: name of the template repository owner
    /// - `template_repo`: name of the template repository
    /// - `body`: See [`GenerateRepoOption`]
    pub fn generate_repo(
        &self,
        template_owner: &str,
        template_repo: &str,
        body: GenerateRepoOption,
    ) -> crate::sync::Request<'_, endpoints::GenerateRepo<'_>, Repository> {
        endpoints::GenerateRepo {
            template_owner,
            template_repo,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Get a repository by id
    ///
    /// - `id`: id of the repo to get
    pub fn repo_get_by_id(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::RepoGetById, Repository> {
        endpoints::RepoGetById { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get instance's global settings for api
    pub fn get_general_api_settings(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetGeneralApiSettings, GeneralAPISettings> {
        endpoints::GetGeneralApiSettings {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get instance's global settings for Attachment
    pub fn get_general_attachment_settings(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetGeneralAttachmentSettings, GeneralAttachmentSettings>
    {
        endpoints::GetGeneralAttachmentSettings {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get instance's global settings for repositories
    pub fn get_general_repository_settings(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetGeneralRepositorySettings, GeneralRepoSettings>
    {
        endpoints::GetGeneralRepositorySettings {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get instance's global settings for ui
    pub fn get_general_ui_settings(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetGeneralUiSettings, GeneralUISettings> {
        endpoints::GetGeneralUiSettings {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get default signing-key.gpg
    pub fn get_signing_key(&self) -> crate::sync::Request<'_, endpoints::GetSigningKey, String> {
        endpoints::GetSigningKey {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get default signing-key.ssh
    pub fn get_ssh_signing_key(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetSshSigningKey, String> {
        endpoints::GetSshSigningKey {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a team
    ///
    /// - `id`: id of the team to get
    pub fn org_get_team(&self, id: i64) -> crate::sync::Request<'_, endpoints::OrgGetTeam, Team> {
        endpoints::OrgGetTeam { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a team
    ///
    /// - `id`: id of the team to delete
    pub fn org_delete_team(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::OrgDeleteTeam, ()> {
        endpoints::OrgDeleteTeam { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Edit a team
    ///
    /// - `id`: id of the team to edit
    /// - `body`: See [`EditTeamOption`]
    pub fn org_edit_team(
        &self,
        id: i64,
        body: EditTeamOption,
    ) -> crate::sync::Request<'_, endpoints::OrgEditTeam, Team> {
        endpoints::OrgEditTeam { id, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a team's activity feeds
    ///
    /// - `id`: id of the team
    pub fn org_list_team_activity_feeds(
        &self,
        id: i64,
        query: OrgListTeamActivityFeedsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListTeamActivityFeeds,
        (ActivityFeedsListHeaders, Vec<Activity>),
    > {
        endpoints::OrgListTeamActivityFeeds { id, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a team's members
    ///
    /// - `id`: id of the team
    pub fn org_list_team_members(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::OrgListTeamMembers, (UserListHeaders, Vec<User>)> {
        endpoints::OrgListTeamMembers { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a particular member of team
    ///
    /// - `id`: id of the team
    /// - `username`: username of the member to list
    pub fn org_list_team_member(
        &self,
        id: i64,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListTeamMember<'_>, User> {
        endpoints::OrgListTeamMember { id, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a team member
    ///
    /// - `id`: id of the team
    /// - `username`: username of the user to add
    pub fn org_add_team_member(
        &self,
        id: i64,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgAddTeamMember<'_>, ()> {
        endpoints::OrgAddTeamMember { id, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Remove a team member
    ///
    /// - `id`: id of the team
    /// - `username`: username of the user to remove
    pub fn org_remove_team_member(
        &self,
        id: i64,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgRemoveTeamMember<'_>, ()> {
        endpoints::OrgRemoveTeamMember { id, username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a team's repos
    ///
    /// - `id`: id of the team
    pub fn org_list_team_repos(
        &self,
        id: i64,
    ) -> crate::sync::Request<
        '_,
        endpoints::OrgListTeamRepos,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::OrgListTeamRepos { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a particular repo of team
    ///
    /// - `id`: id of the team
    /// - `org`: organization that owns the repo to list
    /// - `repo`: name of the repo to list
    pub fn org_list_team_repo(
        &self,
        id: i64,
        org: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListTeamRepo<'_>, Repository> {
        endpoints::OrgListTeamRepo { id, org, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a repository to a team
    ///
    /// - `id`: id of the team
    /// - `org`: organization that owns the repo to add
    /// - `repo`: name of the repo to add
    pub fn org_add_team_repository(
        &self,
        id: i64,
        org: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgAddTeamRepository<'_>, ()> {
        endpoints::OrgAddTeamRepository { id, org, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Remove a repository from a team
    ///
    /// - `id`: id of the team
    /// - `org`: organization that owns the repo to remove
    /// - `repo`: name of the repo to remove
    pub fn org_remove_team_repository(
        &self,
        id: i64,
        org: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgRemoveTeamRepository<'_>, ()> {
        endpoints::OrgRemoveTeamRepository { id, org, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search for topics by keyword
    ///
    pub fn topic_search(
        &self,
        query: TopicSearchQuery,
    ) -> crate::sync::Request<'_, endpoints::TopicSearch, TopicSearchResults> {
        endpoints::TopicSearch { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get the authenticated user
    pub fn user_get_current(&self) -> crate::sync::Request<'_, endpoints::UserGetCurrent, User> {
        endpoints::UserGetCurrent {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search for user's action jobs according filter conditions
    ///
    pub fn user_search_run_jobs(
        &self,
        query: UserSearchRunJobsQuery,
    ) -> crate::sync::Request<'_, endpoints::UserSearchRunJobs, Vec<ActionRunJob>> {
        endpoints::UserSearchRunJobs { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an user's actions runner registration token
    pub fn user_get_runner_registration_token(
        &self,
    ) -> crate::sync::Request<'_, endpoints::UserGetRunnerRegistrationToken, RegistrationToken>
    {
        endpoints::UserGetRunnerRegistrationToken {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create or Update a secret value in a user scope
    ///
    /// - `secretname`: name of the secret
    /// - `body`: See [`CreateOrUpdateSecretOption`]
    pub fn update_user_secret(
        &self,
        secretname: &str,
        body: CreateOrUpdateSecretOption,
    ) -> crate::sync::Request<'_, endpoints::UpdateUserSecret<'_>, ()> {
        endpoints::UpdateUserSecret {
            secretname,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a secret in a user scope
    ///
    /// - `secretname`: name of the secret
    pub fn delete_user_secret(
        &self,
        secretname: &str,
    ) -> crate::sync::Request<'_, endpoints::DeleteUserSecret<'_>, ()> {
        endpoints::DeleteUserSecret { secretname }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get the user-level list of variables which is created by current doer
    ///
    pub fn get_user_variables_list(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::GetUserVariablesList,
        (VariableListHeaders, Vec<ActionVariable>),
    > {
        endpoints::GetUserVariablesList {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a user-level variable which is created by current doer
    ///
    /// - `variablename`: name of the variable
    pub fn get_user_variable(
        &self,
        variablename: &str,
    ) -> crate::sync::Request<'_, endpoints::GetUserVariable<'_>, ActionVariable> {
        endpoints::GetUserVariable { variablename }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a user-level variable which is created by current doer
    ///
    /// - `variablename`: name of the variable
    /// - `body`: See [`UpdateVariableOption`]
    pub fn update_user_variable(
        &self,
        variablename: &str,
        body: UpdateVariableOption,
    ) -> crate::sync::Request<'_, endpoints::UpdateUserVariable<'_>, ()> {
        endpoints::UpdateUserVariable {
            variablename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Create a user-level variable
    ///
    /// - `variablename`: name of the variable
    /// - `body`: See [`CreateVariableOption`]
    pub fn create_user_variable(
        &self,
        variablename: &str,
        body: CreateVariableOption,
    ) -> crate::sync::Request<'_, endpoints::CreateUserVariable<'_>, ()> {
        endpoints::CreateUserVariable {
            variablename,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete a user-level variable which is created by current doer
    ///
    /// - `variablename`: name of the variable
    pub fn delete_user_variable(
        &self,
        variablename: &str,
    ) -> crate::sync::Request<'_, endpoints::DeleteUserVariable<'_>, ()> {
        endpoints::DeleteUserVariable { variablename }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the authenticated user's oauth2 applications
    ///
    pub fn user_get_oauth2_applications(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserGetOAuth2Applications,
        (OAuth2ApplicationListHeaders, Vec<OAuth2Application>),
    > {
        endpoints::UserGetOAuth2Applications {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Creates a new OAuth2 application
    ///
    /// - `body`: See [`CreateOAuth2ApplicationOptions`]
    pub fn user_create_oauth2_application(
        &self,
        body: CreateOAuth2ApplicationOptions,
    ) -> crate::sync::Request<'_, endpoints::UserCreateOAuth2Application, OAuth2Application> {
        endpoints::UserCreateOAuth2Application { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get an OAuth2 application
    ///
    /// - `id`: Application ID to be found
    pub fn user_get_oauth2_application(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserGetOAuth2Application, OAuth2Application> {
        endpoints::UserGetOAuth2Application { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete an OAuth2 application
    ///
    /// - `id`: token to be deleted
    pub fn user_delete_oauth2_application(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserDeleteOAuth2Application, ()> {
        endpoints::UserDeleteOAuth2Application { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update an OAuth2 application, this includes regenerating the client secret
    ///
    /// - `id`: application to be updated
    /// - `body`: See [`CreateOAuth2ApplicationOptions`]
    pub fn user_update_oauth2_application(
        &self,
        id: i64,
        body: CreateOAuth2ApplicationOptions,
    ) -> crate::sync::Request<'_, endpoints::UserUpdateOAuth2Application, OAuth2Application> {
        endpoints::UserUpdateOAuth2Application { id, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update avatar of the current user
    ///
    /// - `body`: See [`UpdateUserAvatarOption`]
    pub fn user_update_avatar(
        &self,
        body: UpdateUserAvatarOption,
    ) -> crate::sync::Request<'_, endpoints::UserUpdateAvatar, ()> {
        endpoints::UserUpdateAvatar { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete avatar of the current user. It will be replaced by a default one
    pub fn user_delete_avatar(&self) -> crate::sync::Request<'_, endpoints::UserDeleteAvatar, ()> {
        endpoints::UserDeleteAvatar {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Blocks a user from the doer
    ///
    /// - `username`: username of the user
    pub fn user_block_user(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserBlockUser<'_>, ()> {
        endpoints::UserBlockUser { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List all email addresses of the current user
    pub fn user_list_emails(
        &self,
    ) -> crate::sync::Request<'_, endpoints::UserListEmails, Vec<Email>> {
        endpoints::UserListEmails {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add an email addresses to the current user's account
    ///
    /// - `body`: See [`CreateEmailOption`]
    pub fn user_add_email(
        &self,
        body: CreateEmailOption,
    ) -> crate::sync::Request<'_, endpoints::UserAddEmail, Vec<Email>> {
        endpoints::UserAddEmail { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete email addresses from the current user's account
    ///
    /// - `body`: See [`DeleteEmailOption`]
    pub fn user_delete_email(
        &self,
        body: DeleteEmailOption,
    ) -> crate::sync::Request<'_, endpoints::UserDeleteEmail, ()> {
        endpoints::UserDeleteEmail { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the authenticated user's followers
    ///
    pub fn user_current_list_followers(
        &self,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentListFollowers, (UserListHeaders, Vec<User>)>
    {
        endpoints::UserCurrentListFollowers {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the users that the authenticated user is following
    ///
    pub fn user_current_list_following(
        &self,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentListFollowing, (UserListHeaders, Vec<User>)>
    {
        endpoints::UserCurrentListFollowing {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check whether a user is followed by the authenticated user
    ///
    /// - `username`: username of followed user
    pub fn user_current_check_following(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentCheckFollowing<'_>, ()> {
        endpoints::UserCurrentCheckFollowing { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Follow a user
    ///
    /// - `username`: username of user to follow
    pub fn user_current_put_follow(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentPutFollow<'_>, ()> {
        endpoints::UserCurrentPutFollow { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Unfollow a user
    ///
    /// - `username`: username of user to unfollow
    pub fn user_current_delete_follow(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentDeleteFollow<'_>, ()> {
        endpoints::UserCurrentDeleteFollow { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a Token to verify
    pub fn get_verification_token(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetVerificationToken, String> {
        endpoints::GetVerificationToken {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Verify a GPG key
    ///
    /// - `body`: See [`VerifyGPGKeyOption`]
    pub fn user_verify_gpg_key(
        &self,
        body: VerifyGPGKeyOption,
    ) -> crate::sync::Request<'_, endpoints::UserVerifyGpgKey, GPGKey> {
        endpoints::UserVerifyGpgKey { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the authenticated user's GPG keys
    ///
    pub fn user_current_list_gpg_keys(
        &self,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentListGpgKeys, (GpgKeyListHeaders, Vec<GPGKey>)>
    {
        endpoints::UserCurrentListGpgKeys {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Add a GPG public key to current user's account
    ///
    /// - `Form`: See [`CreateGPGKeyOption`]
    pub fn user_current_post_gpg_key(
        &self,
        form: CreateGPGKeyOption,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentPostGpgKey, GPGKey> {
        endpoints::UserCurrentPostGpgKey { body: form }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a GPG key
    ///
    /// - `id`: id of key to get
    pub fn user_current_get_gpg_key(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentGetGpgKey, GPGKey> {
        endpoints::UserCurrentGetGpgKey { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Remove a GPG public key from current user's account
    ///
    /// - `id`: id of key to delete
    pub fn user_current_delete_gpg_key(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentDeleteGpgKey, ()> {
        endpoints::UserCurrentDeleteGpgKey { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the authenticated user's webhooks
    ///
    pub fn user_list_hooks(&self) -> crate::sync::Request<'_, endpoints::UserListHooks, Vec<Hook>> {
        endpoints::UserListHooks {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a hook
    ///
    /// - `body`: See [`CreateHookOption`]
    pub fn user_create_hook(
        &self,
        body: CreateHookOption,
    ) -> crate::sync::Request<'_, endpoints::UserCreateHook, Hook> {
        endpoints::UserCreateHook { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a hook
    ///
    /// - `id`: id of the hook to get
    pub fn user_get_hook(&self, id: i64) -> crate::sync::Request<'_, endpoints::UserGetHook, Hook> {
        endpoints::UserGetHook { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a hook
    ///
    /// - `id`: id of the hook to delete
    pub fn user_delete_hook(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserDeleteHook, ()> {
        endpoints::UserDeleteHook { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update a hook
    ///
    /// - `id`: id of the hook to update
    /// - `body`: See [`EditHookOption`]
    pub fn user_edit_hook(
        &self,
        id: i64,
        body: EditHookOption,
    ) -> crate::sync::Request<'_, endpoints::UserEditHook, Hook> {
        endpoints::UserEditHook { id, body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the authenticated user's public keys
    ///
    pub fn user_current_list_keys(
        &self,
        query: UserCurrentListKeysQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserCurrentListKeys,
        (PublicKeyListHeaders, Vec<PublicKey>),
    > {
        endpoints::UserCurrentListKeys { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a public key
    ///
    /// - `body`: See [`CreateKeyOption`]
    pub fn user_current_post_key(
        &self,
        body: CreateKeyOption,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentPostKey, PublicKey> {
        endpoints::UserCurrentPostKey { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a public key
    ///
    /// - `id`: id of key to get
    pub fn user_current_get_key(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentGetKey, PublicKey> {
        endpoints::UserCurrentGetKey { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Delete a public key
    ///
    /// - `id`: id of key to delete
    pub fn user_current_delete_key(
        &self,
        id: i64,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentDeleteKey, ()> {
        endpoints::UserCurrentDeleteKey { id }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the authenticated user's blocked users
    ///
    pub fn user_list_blocked_users(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListBlockedUsers,
        (BlockedUserListHeaders, Vec<BlockedUser>),
    > {
        endpoints::UserListBlockedUsers {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the current user's organizations
    ///
    pub fn org_list_current_user_orgs(
        &self,
    ) -> crate::sync::Request<'_, endpoints::OrgListCurrentUserOrgs, Vec<Organization>> {
        endpoints::OrgListCurrentUserOrgs {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get quota information for the authenticated user
    pub fn user_get_quota(&self) -> crate::sync::Request<'_, endpoints::UserGetQuota, QuotaInfo> {
        endpoints::UserGetQuota {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the artifacts affecting the authenticated user's quota
    ///
    pub fn user_list_quota_artifacts(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListQuotaArtifacts,
        (QuotaUsedArtifactListHeaders, Vec<QuotaUsedArtifact>),
    > {
        endpoints::UserListQuotaArtifacts {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the attachments affecting the authenticated user's quota
    ///
    pub fn user_list_quota_attachments(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListQuotaAttachments,
        (QuotaUsedAttachmentListHeaders, Vec<QuotaUsedAttachment>),
    > {
        endpoints::UserListQuotaAttachments {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if the authenticated user is over quota for a given subject
    ///
    pub fn user_check_quota(
        &self,
        query: UserCheckQuotaQuery,
    ) -> crate::sync::Request<'_, endpoints::UserCheckQuota, bool> {
        endpoints::UserCheckQuota { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the packages affecting the authenticated user's quota
    ///
    pub fn user_list_quota_packages(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListQuotaPackages,
        (QuotaUsedPackageListHeaders, Vec<QuotaUsedPackage>),
    > {
        endpoints::UserListQuotaPackages {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the repos that the authenticated user owns
    ///
    pub fn user_current_list_repos(
        &self,
        query: UserCurrentListReposQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserCurrentListRepos,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::UserCurrentListRepos { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Create a repository
    ///
    /// - `body`: See [`CreateRepoOption`]
    pub fn create_current_user_repo(
        &self,
        body: CreateRepoOption,
    ) -> crate::sync::Request<'_, endpoints::CreateCurrentUserRepo, Repository> {
        endpoints::CreateCurrentUserRepo { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get current user's account settings
    pub fn get_user_settings(
        &self,
    ) -> crate::sync::Request<'_, endpoints::GetUserSettings, UserSettings> {
        endpoints::GetUserSettings {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Update settings in current user's account
    ///
    /// - `body`: See [`UserSettingsOptions`]
    pub fn update_user_settings(
        &self,
        body: UserSettingsOptions,
    ) -> crate::sync::Request<'_, endpoints::UpdateUserSettings, UserSettings> {
        endpoints::UpdateUserSettings { body: body }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// The repos that the authenticated user has starred
    ///
    pub fn user_current_list_starred(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserCurrentListStarred,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::UserCurrentListStarred {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Whether the authenticated is starring the repo
    ///
    /// - `owner`: owner of the repo
    /// - `repo`: name of the repo
    pub fn user_current_check_starring(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentCheckStarring<'_>, ()> {
        endpoints::UserCurrentCheckStarring { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Star the given repo
    ///
    /// - `owner`: owner of the repo to star
    /// - `repo`: name of the repo to star
    pub fn user_current_put_star(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentPutStar<'_>, ()> {
        endpoints::UserCurrentPutStar { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Unstar the given repo
    ///
    /// - `owner`: owner of the repo to unstar
    /// - `repo`: name of the repo to unstar
    pub fn user_current_delete_star(
        &self,
        owner: &str,
        repo: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCurrentDeleteStar<'_>, ()> {
        endpoints::UserCurrentDeleteStar { owner, repo }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get list of all existing stopwatches
    ///
    pub fn user_get_stop_watches(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserGetStopWatches,
        (StopWatchListHeaders, Vec<StopWatch>),
    > {
        endpoints::UserGetStopWatches {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List repositories watched by the authenticated user
    ///
    pub fn user_current_list_subscriptions(
        &self,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserCurrentListSubscriptions,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::UserCurrentListSubscriptions {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List all the teams a user belongs to
    ///
    pub fn user_list_teams(
        &self,
    ) -> crate::sync::Request<'_, endpoints::UserListTeams, (TeamListHeaders, Vec<Team>)> {
        endpoints::UserListTeams {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the current user's tracked times
    ///
    pub fn user_current_tracked_times(
        &self,
        query: UserCurrentTrackedTimesQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserCurrentTrackedTimes,
        (TrackedTimeListHeaders, Vec<TrackedTime>),
    > {
        endpoints::UserCurrentTrackedTimes { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Unblocks a user from the doer
    ///
    /// - `username`: username of the user
    pub fn user_unblock_user(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserUnblockUser<'_>, ()> {
        endpoints::UserUnblockUser { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Search for users
    ///
    pub fn user_search(
        &self,
        query: UserSearchQuery,
    ) -> crate::sync::Request<'_, endpoints::UserSearch, UserSearchResults> {
        endpoints::UserSearch { query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a user
    ///
    /// - `username`: username of user to get
    pub fn user_get(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserGet<'_>, User> {
        endpoints::UserGet { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a user's activity feeds
    ///
    /// - `username`: username of user
    pub fn user_list_activity_feeds(
        &self,
        username: &str,
        query: UserListActivityFeedsQuery,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListActivityFeeds<'_>,
        (ActivityFeedsListHeaders, Vec<Activity>),
    > {
        endpoints::UserListActivityFeeds { username, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the given user's followers
    ///
    /// - `username`: username of user
    pub fn user_list_followers(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserListFollowers<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::UserListFollowers { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the users that the given user is following
    ///
    /// - `username`: username of user
    pub fn user_list_following(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserListFollowing<'_>, (UserListHeaders, Vec<User>)>
    {
        endpoints::UserListFollowing { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Check if one user is following another user
    ///
    /// - `username`: username of following user
    /// - `target`: username of followed user
    pub fn user_check_following(
        &self,
        username: &str,
        target: &str,
    ) -> crate::sync::Request<'_, endpoints::UserCheckFollowing<'_>, ()> {
        endpoints::UserCheckFollowing { username, target }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the given user's GPG keys
    ///
    /// - `username`: username of user
    pub fn user_list_gpg_keys(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserListGpgKeys<'_>, (GpgKeyListHeaders, Vec<GPGKey>)>
    {
        endpoints::UserListGpgKeys { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get a user's heatmap
    ///
    /// - `username`: username of user to get
    pub fn user_get_heatmap_data(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::UserGetHeatmapData<'_>, Vec<UserHeatmapData>> {
        endpoints::UserGetHeatmapData { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the given user's public keys
    ///
    /// - `username`: username of user
    pub fn user_list_keys(
        &self,
        username: &str,
        query: UserListKeysQuery,
    ) -> crate::sync::Request<'_, endpoints::UserListKeys<'_>, (PublicKeyListHeaders, Vec<PublicKey>)>
    {
        endpoints::UserListKeys { username, query }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List a user's organizations
    ///
    /// - `username`: username of user
    pub fn org_list_user_orgs(
        &self,
        username: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgListUserOrgs<'_>, Vec<Organization>> {
        endpoints::OrgListUserOrgs { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Get user permissions in organization
    ///
    /// - `username`: username of user
    /// - `org`: name of the organization
    pub fn org_get_user_permissions(
        &self,
        username: &str,
        org: &str,
    ) -> crate::sync::Request<'_, endpoints::OrgGetUserPermissions<'_>, OrganizationPermissions>
    {
        endpoints::OrgGetUserPermissions { username, org }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the repos owned by the given user
    ///
    /// - `username`: username of user
    pub fn user_list_repos(
        &self,
        username: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListRepos<'_>,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::UserListRepos { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// The repos that the given user has starred
    ///
    /// - `username`: username of user
    pub fn user_list_starred(
        &self,
        username: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListStarred<'_>,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::UserListStarred { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the repositories watched by a user
    ///
    /// - `username`: username of the user
    pub fn user_list_subscriptions(
        &self,
        username: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserListSubscriptions<'_>,
        (RepositoryListHeaders, Vec<Repository>),
    > {
        endpoints::UserListSubscriptions { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// List the specified user's access tokens
    ///
    /// - `username`: username of user
    pub fn user_get_tokens(
        &self,
        username: &str,
    ) -> crate::sync::Request<
        '_,
        endpoints::UserGetTokens<'_>,
        (AccessTokenListHeaders, Vec<AccessToken>),
    > {
        endpoints::UserGetTokens { username }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Generate an access token for the specified user
    ///
    /// - `username`: username of user
    /// - `body`: See [`CreateAccessTokenOption`]
    pub fn user_create_token(
        &self,
        username: &str,
        body: CreateAccessTokenOption,
    ) -> crate::sync::Request<'_, endpoints::UserCreateToken<'_>, AccessToken> {
        endpoints::UserCreateToken {
            username,
            body: body,
        }
        .make_request()
        .wrap_sync::<_, _>(self)
    }

    /// Delete an access token from the specified user's account
    ///
    /// - `username`: username of user
    /// - `token`: token to be deleted, identified by ID and if not available by name
    pub fn user_delete_access_token(
        &self,
        username: &str,
        token: &str,
    ) -> crate::sync::Request<'_, endpoints::UserDeleteAccessToken<'_>, ()> {
        endpoints::UserDeleteAccessToken { username, token }
            .make_request()
            .wrap_sync::<_, _>(self)
    }

    /// Returns the version of the running application
    pub fn get_version(&self) -> crate::sync::Request<'_, endpoints::GetVersion, ServerVersion> {
        endpoints::GetVersion {}
            .make_request()
            .wrap_sync::<_, _>(self)
    }
}
