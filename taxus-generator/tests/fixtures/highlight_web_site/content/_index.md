+++
title = "Web Language Highlighting"
+++

JavaScript, via the `js` alias:

```js
const greet = (name) => {
  return `Hello, ${name}!`;
};
```

CSS:

```css
.card {
  color: #333;
  margin: 0 auto;
}
```

HTML with embedded script and style — the injections make the
JavaScript and CSS inside highlight as themselves:

```html
<button class="counter" onclick="bump()">0</button>
<script>
  function bump() {
    const n = document.querySelector(".counter");
    n.textContent = String(Number(n.textContent) + 1);
  }
</script>
<style>
  .counter { font-weight: bold; }
</style>
```

An inline style attribute:

```html
<p style="color: red; margin: 0;">Hello</p>
```

And an unknown language still degrades to plain escaped text:

```brainfuck
++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]
```
