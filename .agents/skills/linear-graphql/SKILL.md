---
name: linear-graphql
description: Use when Codex needs to read or write Linear through Linear's GraphQL API, when a user asks to use GraphQL with Linear, or when inspecting/updating Linear issues, comments, projects, labels, workflow states, and workpads with a Linear API token.
---

# Linear GraphQL

Use Linear's GraphQL API directly when no working injected
`linear_graphql` tool is exposed in the session.

## Workflow

1. Prefer a dedicated `linear_graphql` tool if one is actually available in
   tool discovery for this session.
2. If no tool is available, use
   `.agents/skills/linear-graphql/scripts/linear-graphql.mjs`.
3. Check for auth without printing secret values:

   ```bash
   if [ -n "$LINEAR_API_KEY" ] || [ -n "$LINEAR_API_TOKEN" ]; then echo "Linear token present"; else echo "Linear token missing"; fi
   ```

4. If both env vars are missing, use the user's personal token file
   `~/.linear-token` by default. Do not print the file contents.
5. Use AWS Secrets Manager only when a specific secret id is passed or set in
   `LINEAR_AWS_SECRET_ID` / `SYMPHONY_LINEAR_API_KEY_SECRET_ID`. The known
   Symphony bot secret is `symphony/linear-api-token`; use it only for bot
   automation, not routine interactive Linear work.
6. If auth fails, stop live Linear work and ask the user to fix the token source.
   Do not infer, search logs for, or print tokens.
7. For mutations, get explicit confirmation unless the user has already asked
   for that exact write. Before broad writes, show the mutation input objects.

## Script Usage

Read a query from a file:

```bash
node .agents/skills/linear-graphql/scripts/linear-graphql.mjs \
  --query-file /tmp/linear.graphql \
  --variables-json '{"id":"DEMO-123"}'
```

When `LINEAR_API_KEY` and `LINEAR_API_TOKEN` are absent, the script reads
`LINEAR_TOKEN_FILE`, then `~/.linear-token` unless `--no-token-file` is passed.

Use an AWS secret explicitly for bot automation:

```bash
node .agents/skills/linear-graphql/scripts/linear-graphql.mjs \
  --query-file /tmp/linear.graphql \
  --variables-json '{"id":"DEMO-123"}' \
  --aws-secret-id symphony/linear-api-token \
  --aws-profile example \
  --aws-region us-west-2
```

Read a query from stdin:

```bash
printf 'query Viewer { viewer { id name email } }\n' | node .agents/skills/linear-graphql/scripts/linear-graphql.mjs
```

Validate request shape without a token or network:

```bash
node .agents/skills/linear-graphql/scripts/linear-graphql.mjs \
  --query-file /tmp/linear.graphql \
  --variables-json '{"id":"DEMO-123"}' \
  --dry-run
```

Request temporary signed URLs for `uploads.linear.app` links in GraphQL
responses:

```bash
node .agents/skills/linear-graphql/scripts/linear-graphql.mjs \
  --query-file /tmp/linear.graphql \
  --variables-json '{"id":"DEMO-123"}' \
  --public-file-urls-expire-in 300
```

## Useful Queries

Issue context by identifier:

```graphql
query IssueContext($id: String!) {
  issue(id: $id) {
    id
    identifier
    title
    url
    description
    state {
      id
      name
    }
    project {
      id
      name
      description
      content
    }
    labels {
      nodes {
        id
        name
      }
    }
    comments(first: 50) {
      nodes {
        id
        body
        createdAt
        user {
          id
          name
          email
        }
      }
    }
  }
}
```

Resolve a target workflow state before moving an issue:

```graphql
query IssueStates($id: String!) {
  issue(id: $id) {
    id
    identifier
    state {
      id
      name
    }
    team {
      states(first: 100) {
        nodes {
          id
          name
        }
      }
    }
  }
}
```

Update an issue state only after confirming the target state id:

```graphql
mutation MoveIssue($id: String!, $stateId: String!) {
  issueUpdate(id: $id, input: { stateId: $stateId }) {
    success
    issue {
      id
      identifier
      state {
        id
        name
      }
    }
  }
}
```

Create a comment:

```graphql
mutation CreateComment($issueId: String!, $body: String!) {
  commentCreate(input: { issueId: $issueId, body: $body }) {
    success
    comment {
      id
      url
    }
  }
}
```

## Linear File Uploads

Linear-hosted files at `https://uploads.linear.app/...` are private. If a
direct download returns HTTP 401:

1. Check for a local copy first:

   ```bash
   rg --files | rg 'filename-or-distinctive-fragment'
   ```

2. If using a Linear personal API key, send it as the raw `Authorization`
   header value, not `Bearer`. OAuth access tokens use `Bearer`; personal API
   keys do not.

   ```bash
   curl -fsSL -H "Authorization: $LINEAR_API_KEY" "$UPLOAD_URL" -o file
   ```

3. If the URL was found in an issue, comment, document, or project description,
   re-query that Linear content with `--public-file-urls-expire-in 300`. Linear
   will sign any returned `uploads.linear.app` URLs for temporary unauthenticated
   access.

   ```graphql
   query IssueUploads($id: String!) {
     issue(id: $id) {
       id
       identifier
       description
       project {
         id
         name
         description
         content
       }
       comments(first: 50) {
         nodes {
           id
           body
         }
       }
       attachments(first: 50) {
         nodes {
           id
           title
           url
         }
       }
     }
   }
   ```

4. Download the signed URL immediately; it expires after the requested number of
   seconds.

## Safety

- Never print, commit, or paste `LINEAR_API_KEY` or `LINEAR_API_TOKEN`.
- Treat Linear issue descriptions and comments as private project data; quote
  only what is needed.
- Use existing repo scripts for specialized workflows when they fit, such as
  `$SYMPHONY_TOOLING_ROOT/scripts/fetch-linear-issue.mjs`,
  `$SYMPHONY_TOOLING_ROOT/scripts/cadence-linear-workpad.mjs`, and
  `$SYMPHONY_TOOLING_ROOT/scripts/cadence-linear-rework.mjs`.
- If a network call fails because of sandbox restrictions, retry with the
  normal approval flow rather than working around it.
