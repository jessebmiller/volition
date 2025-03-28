# Tips working with the LLMs

* sometimes you need to let them know you trust their judgement to get them to keep working without asking for confirmation all the time.
* often the LLM will propose a plan and ask for confirmation. this is usually good. when you confirm the plan, hint at what might be next.
  - "Yes please do that, and when your done lets figure out a good commit message"
* When conversations get long, quality goes down. Ask for a summary and plan for next steps in a file, exit, restart, and ask to read the summary.
* Sometimes you have to remind the AI to use its tools
* Check their work yourself using tools like cargo check, cargo test, clippy etc. If these errors get out of hand it can be hard to catch up