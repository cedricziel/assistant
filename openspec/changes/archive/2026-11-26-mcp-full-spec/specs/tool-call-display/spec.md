# Spec Delta

## ADDED Requirements

### Requirement: Embedded MCP App
A tool-call chip whose call carries an MCP App SHALL render the app frame directly below the chip, visibly marked as third-party content from the named server, and SHALL keep rendering it when the conversation is reloaded.

#### Scenario: Reload a conversation with an app
- **WHEN** a conversation containing a tool call with an app is reopened
- **THEN** the app frame renders below that tool call's chip with its server named
