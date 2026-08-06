-- sqlfluff:dialect:tsql

SET ANSI_NULLS ON;
SET QUOTED_IDENTIFIER ON;
SET NOCOUNT ON;
SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED;

DECLARE @user_name NVARCHAR(200);

SELECT @user_name = N'Ego Hygiene';

SELECT
    users.user_id,
    users.display_name
FROM dbo.users AS users
WHERE users.display_name = @user_name;
