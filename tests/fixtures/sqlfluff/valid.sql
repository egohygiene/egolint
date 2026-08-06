-- sqlfluff:dialect:postgres

SELECT
    app_users.user_id,
    app_users.display_name,
    app_users.created_at
FROM application_users AS app_users
WHERE
    app_users.is_active = TRUE
    AND app_users.deleted_at IS NULL
ORDER BY
    app_users.created_at DESC;
