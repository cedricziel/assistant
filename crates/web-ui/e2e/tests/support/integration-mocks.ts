import { Page, Route } from "@playwright/test";

/**
 * Mocks and fixtures shared by the integrations visual tests: identity,
 * spaces, and every integrations endpoint the screens call.
 */

export const STAMP = "2026-01-01T09:00:00Z";
export const ME = "user-1";

export type Role = "org_admin" | "space_admin" | "member";

export const SPACES = [
  { id: "eng", name: "Engineering", slug: "eng" },
  { id: "ops", name: "Operations", slug: "ops" },
];

export function integration(
  id: string,
  name: string,
  extra: Record<string, unknown> = {},
) {
  return {
    id,
    layer: "org",
    slug: id.replace(/^int_/, "").replace(/_/g, "-"),
    name,
    connect_url: `https://mcp.example.com/${id}`,
    auth_kind: "header",
    credential_mode: "shared",
    trust: "confirm",
    enabled: true,
    status: "ready",
    has_credential: true,
    created_at: STAMP,
    updated_at: STAMP,
    ...extra,
  };
}

export const ORG_INTEGRATIONS = [
  integration("int_linear", "Linear", {
    auth_kind: "oauth",
    credential_mode: "per_user",
    status: "needs_connection",
    has_credential: false,
  }),
  integration("int_github", "GitHub"),
  integration("int_sentry", "Sentry", {
    enabled: false,
    status: "disabled",
  }),
  integration("int_notion", "Notion", {
    auth_kind: "oauth",
    status: "auth_expired",
  }),
];

export const SPACE_INTEGRATIONS = [
  integration("int_pagerduty", "PagerDuty", {
    layer: "space",
    space_id: "eng",
    status: "unreachable",
  }),
  integration("int_figma", "Figma", {
    layer: "space",
    space_id: "eng",
    credential_mode: "per_user",
    status: "needs_connection",
    has_credential: false,
  }),
];

export const PERSONAL_INTEGRATIONS = [
  integration("int_my_notion", "Notion (personal)", {
    layer: "user",
    owner_user_id: ME,
    credential_mode: "per_user",
    auth_kind: "oauth",
    status: "needs_connection",
    has_credential: false,
  }),
  integration("int_my_linear", "Linear (personal)", {
    layer: "user",
    owner_user_id: ME,
    credential_mode: "per_user",
    auth_kind: "oauth",
    status: "blocked_by_policy",
    enabled: false,
  }),
];

export const CATALOG = [
  {
    id: "curated/linear-app-mcp",
    name: "Linear",
    description: "Issue tracking and project planning for product teams.",
    domain: "linear.app",
    categories: ["project-management", "dev"],
    connect_url: "https://mcp.linear.app/mcp",
    auth: { kind: "oauth" },
    installable_layers: ["org", "space", "user"],
  },
  {
    id: "curated/github-mcp",
    name: "GitHub",
    description: "Repositories, pull requests and issues.",
    domain: "github.com",
    categories: ["dev"],
    connect_url: "https://api.githubcopilot.com/mcp/",
    auth: {
      kind: "header",
      header_name: "Authorization",
      placeholder: "pat",
      value_prefix: "Bearer ",
    },
    installable_layers: ["org", "space", "user"],
  },
];

export type Mocks = {
  role: Role;
  /** Integrations the caller can see, in every layer. */
  integrations?: unknown[];
  policy?: Record<string, unknown>;
};

/**
 * Mock the identity, the spaces and every integrations endpoint the layer
 * screens call, and pre-select the Acme org with the Engineering space.
 */
export async function mockEverything(page: Page, mocks: Mocks) {
  const all = (mocks.integrations ?? []) as Record<string, any>[];
  const policy = {
    catalog_mode: "all",
    allowed_catalog_ids: [],
    allow_custom_urls: false,
    allow_user_integrations: true,
    ...mocks.policy,
  };

  await page.addInitScript(() => {
    window.localStorage.setItem(
      "assistant.spaceSelection",
      JSON.stringify({
        orgId: "org-1",
        orgName: "Acme",
        spaceId: "eng",
        spaceName: "Engineering",
      }),
    );
  });

  const json = (route: Route, body: unknown, status = 200) =>
    route.fulfill({ status, json: body });

  await page.route(/\/api\/users\/me$/, (route) =>
    json(route, {
      id: ME,
      email: "ada@example.com",
      name: "Ada",
      org_id: "org-1",
      created_at: STAMP,
      updated_at: STAMP,
    }),
  );
  await page.route(/\/api\/orgs$/, (route) =>
    json(route, [
      { id: "org-1", name: "Acme", slug: "acme", auth_mode: "password" },
    ]),
  );
  await page.route(/\/api\/orgs\/org-1$/, (route) =>
    json(route, {
      id: "org-1",
      name: "Acme",
      slug: "acme",
      auth_mode: "password",
      created_at: STAMP,
      updated_at: STAMP,
    }),
  );
  await page.route(/\/api\/orgs\/org-1\/spaces$/, (route) =>
    json(route, SPACES),
  );
  await page.route(/\/api\/orgs\/org-1\/spaces\/([^/]+)\/members$/, (route) => {
    const spaceId = /spaces\/([^/]+)\/members/.exec(route.request().url())![1];
    // Org admins hold the role in the first space; others only in Engineering.
    const role =
      mocks.role === "org_admin" && spaceId !== "eng" ? "member" : mocks.role;
    return json(route, [
      { space_id: spaceId, user_id: ME, role, created_at: STAMP },
    ]);
  });

  await page.route(/\/api\/integrations\/catalog(\?.*)?$/, (route) =>
    json(route, { items: CATALOG }),
  );
  await page.route(/\/api\/integrations\/policy$/, (route) =>
    json(route, policy),
  );
  await page.route(/\/api\/integrations\/int_[a-z_]+$/, (route) => {
    const id = /integrations\/(int_[a-z_]+)$/.exec(route.request().url())![1];
    const found = all.find((i) => i.id === id);
    if (!found) return json(route, { error: "not found" }, 404);
    const optedOut =
      id === "int_zendesk" ? [{ space_id: "eng", mode: "opt_out" }] : [];
    return json(route, { ...found, space_access: optedOut });
  });
  await page.route(/\/api\/integrations(\?.*)?$/, (route) => {
    const request = route.request();
    if (request.method() === "POST") {
      return json(
        route,
        integration("int_new", "Linear", {
          layer: "user",
          owner_user_id: ME,
          auth_kind: "oauth",
          credential_mode: "per_user",
          status: "needs_connection",
          has_credential: false,
          space_access: [],
        }),
        201,
      );
    }
    const url = new URL(request.url());
    const layer = url.searchParams.get("layer");
    const space = url.searchParams.get("space_id");
    let items = all;
    if (layer) items = items.filter((i) => i.layer === layer);
    if (space) {
      // A space lists its own integrations and the org ones offered to it,
      // leaving out the ones it opted out of.
      items = items.filter(
        (i) =>
          (i.layer === "space" && i.space_id === space) ||
          (i.layer === "org" && i.id !== "int_zendesk"),
      );
    }
    return json(route, { items });
  });
}
