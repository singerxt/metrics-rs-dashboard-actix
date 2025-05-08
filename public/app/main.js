import { html, render, useEffect } from "https://esm.sh/htm/preact/standalone";
import MetricBuffer from "./common/MetricBuffer.js";
import PrometheusImport from "./common/PrometheusImport.js";

const buff = new MetricBuffer(10);

let x = 0;
setInterval(async () => {
  if (x > 20) {
    return;
  }
  x++;
	const prom = new PrometheusImport("./prometheus");
	const metrics = await prom.fetchMetrics();
	buff.addMetrics(metrics);
	buff.logMetrics();
}, 1000);
function App(props) {
	return html`<h1>Hello ${props.name}!</h1>`;
}

render(html`<${App} name="World" />`, document.body);
