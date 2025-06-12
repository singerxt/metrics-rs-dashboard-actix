import parsePrometheusTextFormat from "https://cdn.jsdelivr.net/npm/parse-prometheus-text-format@1.1.1/+esm";

/**
 * PrometheusImport class for fetching metrics from a Prometheus endpoint.
 */
class PrometheusImport {
  /**
   * Creates a new PrometheusImport instance.
   * @param {string} url - The URL of the Prometheus metrics endpoint.
   */
  constructor(url) {
    this.prometheusEndpoint = url;
  }

  /**
   * Fetches metrics from the Prometheus endpoint.
   * @returns {Promise<Array>} A promise that resolves to the parsed metrics data.
   * @throws {Error} If the fetch operation fails.
   */
  async fetchMetrics() {
    const response = await fetch(this.prometheusEndpoint);
    if (!response.ok) {
      throw new Error(`Failed to fetch metrics: ${response.statusText}`);
    }
    const text = await response.text();
    const header = response.headers.get("x-dashboard-metrics-unit");
    let units = {};
    try {
      if (header) {
        units = JSON.parse(header);
      }
    } catch (error) {
      console.warn("Failed to parse metrics units header:", error);
    }

    const promJson = parsePrometheusTextFormat(text);

    for (const sample of promJson) {
      sample.unit = units && units[sample.name] ? units[sample.name] : "count";
    }

    return promJson;
  }
}

export default PrometheusImport;
