# Keep All Sessions

I find myself wanting to start new sessions for new features or when
the ai gets out of hand, but like the chat apps, let's keep all
sessions around for posterity at least.

It would be nice to be able to go recover any session.

## open questions

- Will this cause problems from the AI being woefully out of context?
  - the code they've just been reading will often be very wrong
  - maybe we keep a checkpoint of the conversation associated with
    every git commit the agent makes. We'd then be able to see what
    work went into development at any given point, and fork from any
    given point with the benefit of the conversation.

## UX

- Probably something closely related to git honestly if we are forking
  and committing conversations to stay parallel to git forking and
  committing.

- I think we'll want to save conversation states between git commits
  too though. those will be the most valuable places to pick up the
  thread. those threads wtih unfinished work.

- We'll want to show conversations in context of their branching
  though.