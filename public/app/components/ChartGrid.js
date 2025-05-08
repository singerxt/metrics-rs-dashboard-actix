import { html } from "https://esm.sh/htm/preact/standalone";
import MetricBuffer from "../common/MetricBuffer.js";
import PrometheusImport from "../common/PrometheusImport.js";

const metricBuffer = new MetricBuffer(10);
const prometheusImporter = new PrometheusImport('./metrics');

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
