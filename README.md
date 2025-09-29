# pren
**pren** is a prompt engine focused on reusability and composability.

## Overview
pren is a powerful prompt management system that allows you to organize, store, and use prompts with advanced template capabilities for dynamic content generation. It provides a command-line interface for managing prompts and integrating them with LLMs for content generation.

## Features
- **Prompt Storage**: File-based storage for organizing and managing prompts
- **Template Variables**: Support for variable substitution in prompts using `{{variable}}` syntax
- **Prompt Composition**: Reference other prompts using `{{prompt:name}}` syntax to build complex prompt structures
- **Dynamic Prompt References**: Variable prompt references with `{{prompt_var:name}}` for dynamic prompt selection
- **LLM Integration**: Generate content directly with LLMs using your prompts
- **CLI Auto-completion**: Tab completion for prompt names and arguments

## Usage
The pren CLI provides several subcommands for managing and using prompts:

### Add a new prompt
```bash
pren add -n greeting -d "A simple greeting" -t general,template -c "Hello, {{name}}!"
```

### List all prompts
```bash
pren list
```

### Show a prompt
```bash
pren show -n greeting
```

### Render a prompt with variables
```bash
pren render -n greeting -a name=World
```

### Render and copy to clipboard
```bash
pren get -n greeting -a name=World
```

### Generate content with LLM
```bash
pren generate -g greeting -a name=World
```

### Delete a prompt
```bash
pren delete -n greeting
```

## Template Syntax
pren supports several template syntaxes for dynamic content generation:

- `{{variable}}`: Replace with the value of the variable passed during rendering
- `{{prompt:name}}`: Include the content of another prompt (with variable substitution if applicable)
- `{{prompt_var:name}}`: Include the content of a prompt specified by the variable `name`
- `{{{{literal}}}}`: Escaped braces render as literal `{{literal}}`

## Examples

### Simple Prompt
```bash
pren add -n hello -c "Hello, world!"
pren render -n hello
# Output: Hello, world!
```

### Prompt with Variables
```bash
pren add -n greeting -c "Hello, {{name}}! Welcome to {{place}}."
pren render -n greeting -a name=Alice,place=pren
# Output: Hello, Alice! Welcome to pren.
```

### Prompt with References
```bash
pren add -n greeting -c "Hello, {{name}}!"
pren add -n formal -c "{{prompt:greeting}} Please enjoy your stay."
pren render -n formal -a name=Bob
# Output: Hello, Bob! Please enjoy your stay.
```

### Prompt with Dynamic References
```bash
pren add -n greeting -c "Hello, {{name}}!"
pren add -n farewell -c "Goodbye, {{name}}!"
pren add -n dynamic -c "Message: {{prompt_var:message_type}}"
pren render -n dynamic -a message_type=greeting,name=Charlie
# Output: Message: Hello, Charlie!
```

## Commands
- `add`: Add a new prompt with name, description, tags, and content
- `show`: Display a prompt's details
- `render`: Render a prompt with provided arguments
- `get`: Render a prompt and copy output to clipboard
- `list`: List all available prompts
- `delete`: Delete a prompt
- `generate`: Render a prompt and generate content with an LLM
- `info`: Show information about the prompt storage
