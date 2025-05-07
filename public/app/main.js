  import { html, render } from 'https://esm.sh/htm/preact/standalone'

  function App (props) {
    return html`<h1>Hello ${props.name}!</h1>`;
  }

  render(html`<${App} name="World" />`, document.body);
