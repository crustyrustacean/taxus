function addCopyButtons(): void {
  for (const code of document.querySelectorAll<HTMLElement>("pre > code")) {
    const block = code.parentElement;
    if (!block || block.querySelector(".copy-code")) {
      continue;
    }

    const button = document.createElement("button");
    button.className = "copy-code";
    button.type = "button";
    button.textContent = "Copy";
    button.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(code.textContent ?? "");
        button.textContent = "Copied";
        window.setTimeout(() => {
          button.textContent = "Copy";
        }, 1500);
      } catch {
        button.textContent = "Copy unavailable";
      }
    });
    block.append(button);
  }
}

addCopyButtons();
