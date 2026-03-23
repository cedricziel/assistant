import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  static values = {
    endpoint: String,
    tokenMasked: String,
    urlMasked: String,
  };

  static targets = ["button", "token", "url", "status"];

  connect() {
    this.revealed = false;
    this.token = null;
    this.url = null;
  }

  toggle() {
    if (this.revealed) {
      this.hideSecrets();
      return;
    }

    if (this.token && this.url) {
      this.showSecrets(this.token, this.url);
      return;
    }

    this.fetchSecrets();
  }

  hideSecrets() {
    this.tokenTarget.textContent = this.tokenMaskedValue;
    this.urlTarget.textContent = this.urlMaskedValue;
    this.buttonTarget.textContent = "Reveal";
    this.buttonTarget.setAttribute("aria-pressed", "false");
    this.statusTarget.textContent = "";
    this.statusTarget.style.color = "#8aa5d8";
    this.revealed = false;
  }

  showSecrets(token, url) {
    this.tokenTarget.textContent = token;
    this.urlTarget.textContent = url;
    this.buttonTarget.textContent = "Hide";
    this.buttonTarget.setAttribute("aria-pressed", "true");
    this.statusTarget.textContent = "Secrets revealed";
    this.statusTarget.style.color = "#8aa5d8";
    this.revealed = true;
  }

  fetchSecrets() {
    this.buttonTarget.disabled = true;
    this.buttonTarget.textContent = "Loading...";
    this.statusTarget.textContent = "Loading secrets...";
    this.statusTarget.style.color = "#8aa5d8";

    fetch(this.endpointValue, {
      method: "GET",
      credentials: "same-origin",
      headers: { Accept: "application/json" },
    })
      .then(function (res) {
        if (!res.ok) {
          throw new Error("failed to load workflow webhook secrets");
        }
        return res.json();
      })
      .then((payload) => {
        if (!payload || !payload.webhook_token || !payload.webhook_url) {
          throw new Error("workflow webhook secret payload is incomplete");
        }
        this.token = payload.webhook_token;
        this.url = payload.webhook_url;
        this.showSecrets(this.token, this.url);
      })
      .catch(() => {
        this.buttonTarget.textContent = "Reveal";
        this.buttonTarget.setAttribute("aria-pressed", "false");
        this.statusTarget.textContent = "Unable to reveal secrets";
        this.statusTarget.style.color = "#ff9b9b";
      })
      .finally(() => {
        this.buttonTarget.disabled = false;
      });
  }
}
