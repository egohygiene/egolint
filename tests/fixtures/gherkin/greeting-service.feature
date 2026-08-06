@integration
Feature: Create a personalized greeting
  The greeting service should create a readable message for a supplied name.

  Scenario: Create a greeting for a named user
    Given the greeting prefix is "Hello"
    When the service creates a greeting for "Ego Hygiene"
    Then the greeting should be "Hello, Ego Hygiene!"

  Scenario: Create a greeting when the name is empty
    Given the greeting prefix is "Hello"
    When the service creates a greeting without a name
    Then the greeting should be "Hello"
