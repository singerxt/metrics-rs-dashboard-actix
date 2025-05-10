import {
  html,
  useEffect,
  useState,
} from "https://esm.sh/htm/preact/standalone";

import MetricBuffer from "../common/MetricBuffer.js";
import PrometheusImport from "../common/PrometheusImport.js";
import CounterChart from "./CounterChart.js";
import GaugeChart from "./GaugeChart.js";
import HistogramChart from "./HistogramChart.js";

const metricBuffer = new MetricBuffer(10);
const prometheusImporter = new PrometheusImport("./prometheus");

const renderChart = (sample) => {
  switch (sample.type) {
    case "COUNTER": {
      return html`<${CounterChart} metricSample=${sample} />`;
    }
    case "GAUGE": {
      return html`<${GaugeChart} metricSample=${sample} />`;
    }
    case "HISTOGRAM": {
      return html`<${HistogramChart} metricSample=${sample} />`;
    }
    default: {
      return html`<h1>Unsupported metric type: ${sample.type}</h1>`;
    }
  }
};

function ChartGrid({ searchValue, refreshRate, bufferSize, pause }) {
  const [metrics, setMetrics] = useState([]);

  useEffect(() => {
    const interval = setInterval(async () => {
      if (pause) {
        return;
      }

      try {
        const metrics = await prometheusImporter.fetchMetrics();
        metricBuffer.addMetrics(metrics);
        const filteredMetrics = metricBuffer.getMetrics().filter((sample) => {
          if (!searchValue) return true;
          return sample.name.toLowerCase().includes(searchValue.toLowerCase());
        });
        setMetrics(filteredMetrics);
      } catch (error) {
        console.error("Error fetching metrics:", error);
        // Optionally display error to user or retry
      }
    }, refreshRate);
    return () => clearInterval(interval);
  }, [refreshRate, searchValue, pause]);

  useEffect(() => {
    metricBuffer.setBufferSize(bufferSize);
  }, [bufferSize]);

  return html`
    <div class="responsive-grid">
      ${metrics.map((sample) => renderChart(sample))}
    </div>
  `;
}

export default ChartGrid;
