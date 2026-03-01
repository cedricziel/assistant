// -- Agent form: skills JSON validator --

(function () {
  var textarea = document.getElementById("skills-json-input");
  if (!textarea) return;

  var errorDiv = document.getElementById("skills-json-error");
  var form = textarea.closest("form");
  if (!form) return;

  function validate() {
    var val = textarea.value.trim();
    errorDiv.textContent = "";
    if (!val) return true;
    try {
      var parsed = JSON.parse(val);
      if (!Array.isArray(parsed)) {
        errorDiv.textContent = "Skills must be a JSON array.";
        return false;
      }
      for (var i = 0; i < parsed.length; i++) {
        var s = parsed[i];
        if (typeof s !== "object" || s === null || Array.isArray(s)) {
          errorDiv.textContent =
            "Each skill must be an object (item " + i + ").";
          return false;
        }
        if (typeof s.id !== "string" || !s.id) {
          errorDiv.textContent =
            "Skill " + i + ' is missing a string "id" field.';
          return false;
        }
        if (typeof s.name !== "string" || !s.name) {
          errorDiv.textContent =
            "Skill " + i + ' is missing a string "name" field.';
          return false;
        }
      }
      return true;
    } catch (e) {
      errorDiv.textContent = "Invalid JSON: " + e.message;
      return false;
    }
  }

  textarea.addEventListener("blur", validate);
  form.addEventListener("submit", function (e) {
    if (!validate()) e.preventDefault();
  });
})();
