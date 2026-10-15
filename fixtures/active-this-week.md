# Sprint

## API

- [ ] Set up database schema in `backend/pov/db.py`
- [ ] Add authentication endpoints — **blocked** on the token spec
- [x] Implement rate limiting
- [x] Add request logging middleware 0
- [ ] Add request logging middleware 1
- [x] Add request logging middleware 2
- [x] Add request logging middleware 3
- [x] Add request logging middleware 5
- [ ] Add request logging middleware 6

### Get something

- [ ] Define the API — see [the OpenAPI spec](https://spec.openapis.org/oas/latest.html)
- [ ] Implement
    - [ ] Sub implement 0 — reuse `parse_items()`
    - [ ] Sub implement 1
    - [ ] Sub implement 2
- [ ] Test

## Frontend

- [x] Create login page
- [ ] Build user dashboard
- [ ] Add error boundaries around `<TaskList />`
- [ ] Write *integration* tests

## Rendering `InlineMarkdown`

- [ ] ~~Pull in react-markdown~~ superseded by the inline tokenizer
- [ ] Keep a literal \*asterisk\* unformatted in task text
- [x] Confirm `a * b * c` and `2*3*4` match CommonMark

## Done (older)

- [x] Bootstrap project repository
- [ ] Configure CI pipeline
- [x] Write API specification
- [x] Set up staging environment
- [x] Migrate legacy data
- [x] Load testing and benchmarks
- [x] Security audit
