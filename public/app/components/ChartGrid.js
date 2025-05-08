import { html } from "https://esm.sh/htm/preact/standalone";

function ChartGrid({ searchValue, refreshRate, bufferSize }) {
  return html`
    <div class="container">
      searchValue ${searchValue}
      refreshRate ${refreshRate}
      bufferSize ${bufferSize}
    </div>
  `;
}

export default ChartGrid;
