# Roadmap

Sync with [#2](https://github.com/Anekoique/Ark/issues/2)

[**Execution Environment and Sandbox**]

- [x] Add sandbox for isolation execution environment. #17
...

[**Lifecycle and Orchestration**]

- [ ] Provide user-defined workflow. Add [workflow] to config.toml.
- [x] Add workspace/worktree support. See [trellis](https://github.com/mindfold-ai/Trellis/tree/main/.trellis/workspace). #8 #9 #14
- [x] Add a spec extraction mechanism through docs/codes to support older projects. [spec-extract](https://github.com/Anekoique/Ark/commit/7d0ae822e1f512d0ff284d7e5dc3bc69b55cf324)
- [x] Add sub-agent support. #15
- [x] Add SPEC constrains audit. [spec-actuators](https://github.com/Anekoique/Ark/commit/cc4de78b49ebb64ef3fbb6f441d40a5dd55b2e96)
...

[**Context and Memory Management**]

- [ ] Better memory(spec and tasks) organization, learn idea [stello](https://github.com/stello-agent/stello/tree/main).
- [ ] Context management, workflow resulted in long contexts currently.
- [ ] Better design/discription of ark-skills, ark-agents, ark-workflow...
...

[**Tool Interface**]

- [ ] Cli extensions for memory management (`ark mem`)...
- [x] Cli tools for Agent invoke directoly without understanding natural language. #3 #5
- [ ] Convenience management to coding-agent settings (cross-platform) with simple cli. Consider a `ark skill add` apply skill to all platforms or manage skill through `./ark/skills`. See [cc-switch](https://github.com/farion1231/cc-switch).
- [ ] Design better description and usage docs for both user and agents.
- [ ] Perf cli tool.
...

[**Platform and Software Engineer**]

- [x] Add support for codex, opencode...  #6 #7
- [ ] Embedded projects and workflow management.
- [ ] Add a new layer (like initiative) to lead project development.
...

[**Veriﬁcation and Evaluation**]

- [ ] Port important benchmarks.
- [ ] Add [system-intelligence-benchmark](https://github.com/sys-intelligence/system-intelligence-benchmark).
...
