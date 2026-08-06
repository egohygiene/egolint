[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateNotNullOrEmpty()]
    [string] $Name
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$normalizedName = $Name.Trim()

if ([string]::IsNullOrWhiteSpace($normalizedName)) {
    throw 'Name must contain at least one non-whitespace character.'
}

[PSCustomObject] @{
    Name     = $normalizedName
    Greeting = "Hello, $normalizedName."
}

