## Summary

-

## Validation

- [ ] `make check`
- [ ] `make test`
- [ ] `make lint`
- [ ] `make format`

## Persona Model Naming Checklist

- [ ] I used canonical terms in docs/UI copy: `Persona`, `Subagent Process`, `A2A Profile`.
- [ ] I avoided unqualified `agent` in architecture prose unless it is a literal code identifier.
- [ ] If legacy identifiers remain, I explained them using canonical terminology in docs/comments.

## Breaking Change Notes

- [ ] This PR intentionally keeps no backward-compatibility layer where migration scope requires removal.
- [ ] I documented any removed/renamed user-facing surface in docs or release notes.
