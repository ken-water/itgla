document.querySelectorAll('.inline-guide[href^="#verify-"]').forEach((link) => {
  link.addEventListener("click", () => {
    const guide = document.querySelector(link.getAttribute("href"));
    if (guide instanceof HTMLDetailsElement) guide.open = true;
  });
});

if (window.location.hash.startsWith("#verify-")) {
  const guide = document.querySelector(window.location.hash);
  if (guide instanceof HTMLDetailsElement) guide.open = true;
}
