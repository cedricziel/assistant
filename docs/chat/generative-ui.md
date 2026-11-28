# Generative UI

In web chat, the assistant can answer with a **surface** instead of prose: a
small card, list, form or choice list that you operate directly. A surface is
built from a fixed set of components. The model chooses the content and
arrangement, but it cannot send code, and nothing in a surface runs as a
script.

A surface appears in the conversation at the point where the assistant created
it, next to the turn that produced it. Opening the conversation later shows it
again from history.

## Where surfaces appear

Only interactive web chat turns can show a surface: a message you send, and
later an edit or a regenerate. Every other client gets the same answer as text:

- voice messages and quick messages (Siri and Shortcuts);
- Slack, Matrix, Signal, Mattermost and Nextcloud;
- the CLI, MCP clients and A2A;
- scheduled tasks;
- work done by a Subagent Process.

The model is not offered the surface tool in those cases, so it writes the
answer in words instead. Every surface also carries plain fallback text. If the
client cannot build the surface, the chat shows that text with an error marker
rather than a broken widget.

## What a surface can hold

A surface uses these components: Text, Image, Icon, Row, Column, List, Card,
Tabs, Divider, Button, CheckBox, TextField, DateTimeInput, ChoicePicker and
Slider. That covers headings and paragraphs, images, simple forms, checkboxes,
pickers and sliders, arranged in columns and rows.

There is no Video, AudioPlayer, Modal, chart or map component, and the model
cannot add one. The component set is enforced on the server before anything is
sent to your browser.

The model can create one surface and update it again in the same turn, for
example to fill a data model after a first render. A surface is complete when
the turn ends.

## Answering a surface

Buttons and fields behave like the rest of the chat. Submitting a form starts a
new user turn with the values you chose. The model sees the surface, the action
name and the submitted values as a structured block, so it can continue from
them.

The user bubble is written by the server, not by the model. It reads
`Submitted “<title>”` and carries a **Submitted via form** marker, so it is
clear that the words are not your own message. A Button may also carry a
`userMessage`; that text is never shown as user speech.

A surface can be answered until your next user message. Once you send anything
after it, the surface becomes read-only and its header is labelled **Closed**.
The server derives this from the conversation rather than storing a closed
state, so a reload shows the same result. A submitted surface is also
read-only and shows the values that were sent.

### Links

A surface can open a link only through `openUrl`, and only when you tap a
Button. The target must be an `https:` or `mailto:` address. Other schemes such
as `javascript:`, `data:` and `http:` are disabled and never reach a prompt.

By default the app asks before opening any link. The prompt shows the full
destination with the host emphasised, and **Open** and **Cancel** buttons.
Nothing opens unless you choose Open. A `mailto:` address always asks, because
there is no host to trust.

An org can mark trusted hosts in `[ui] link_hosts`. Links to those hosts open
without a prompt, matched on the exact host name and ignoring case. The list is
empty by default. If the app cannot load the org's list, it treats it as empty,
so a failure means more prompts, never fewer. See
[Configuration](../operations/configuration.md#generative-ui-surfaces).

### Images

A surface image must be an `https:` URL or an attachment from the same
conversation. The default allows any `https:` host. An org can narrow this with
`[ui] image_hosts`; the server then rejects images from other hosts before the
surface is shown.

Allowing any host means that host can see your IP address and when the image
loaded, in the same way as an image in an email. Orgs that care can restrict
the list to hosts they trust.

## Styling

Surface content is Material on every platform, including iOS and macOS. It
takes its colours and typography from the app's theme, with light and dark both
supported. This is a deliberate v1 exception to the app's adaptive-widget rule:
the surface itself is rendered by a third-party package, and overriding its
controls would mean reimplementing most of the catalog. The frame around the
surface and the link-confirmation prompt do follow the platform's style.

## Related

- [Approvals and questions](approvals-and-questions.md) covers the cards a turn
  can show for tool approvals and questions.
- [Configuration](../operations/configuration.md#generative-ui-surfaces) lists
  the `[ui]` settings.
