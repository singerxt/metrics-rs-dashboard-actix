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
import RateChart from "./RateChart.js";

/**
 * Buffer for storing metric data with a default size of 10
 * @type {MetricBuffer}
 */
const metricBuffer = new MetricBuffer(10);

/**
 * Importer for Prometheus metrics from the ./prometheus endpoint
 * @type {PrometheusImport}
 */
const prometheusImporter = new PrometheusImport("./prometheus");

/**
 * Renders the appropriate chart component based on metric type
 * @param {Object} sample - The metric sample data
 * @returns {JSX.Element} The rendered chart component
 */
const renderChart = (sample) => {
  // Check if this is a rate metric (gauge with _rate_per_sec suffix)
  if (sample.type === "GAUGE" && sample.name.endsWith("_rate_per_sec")) {
    return html`<${RateChart} metricSample=${sample} />`;
  }

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

/**
 * ChartGrid component that displays metric charts in a responsive grid
 * @component
 * @param {Object} props - Component props
 * @param {string} props.searchValue - Text to filter metrics by name
 * @param {number} props.refreshRate - How often to refresh metrics in milliseconds
 * @param {number} props.bufferSize - Size of the metric buffer
 * @param {boolean} props.pause - Whether to pause metric updates
 * @returns {JSX.Element} Rendered grid of metric charts
 */
function ChartGrid({ searchValue, refreshRate, bufferSize, pause }) {
  /**
   * State for storing the filtered metrics
   * @type {[Array, Function]}
   */
  const [metrics, setMetrics] = useState([]);

  /**
   * Effect for fetching and updating metrics at the specified refresh rate
   */
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

  /**
   * Effect for updating the buffer size when it changes
   */
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
