---
name: "pren-format"
description: "pren format with frontmatter metadata"
tags: ["format", "pren", "metadata", "template"]
---
Format the output using the pren templating language:
- Markdown with YAML frontmatter.
- Don't add any unnecessary metadata, only name, description and tags.

Here's an example:
```
---
name: "example-prompt"
description: "A brief description of the prompt"
tags: ["tag1", "tag2"]
---

Here goes the format of the actual prompt.
{{{ It can contain {{input_variables}}, {{prompt:prompt_references}} and {{prompt_var:variable_prompt_references}}. }}}
```
