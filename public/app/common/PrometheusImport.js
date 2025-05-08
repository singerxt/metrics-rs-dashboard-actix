import parsePrometheusTextFormat from 'https://cdn.jsdelivr.net/npm/parse-prometheus-text-format@1.1.1/+esm';

class PrometheusImport {
  constructor(url) {
    this.prometheusEndpoint = url;
  }

  async fetchMetrics() {
    const response = await fetch(this.prometheusEndpoint);
    if (!response.ok) {
      throw new Error(`Failed to fetch metrics: ${response.statusText}`);
    }
    const text = await response.text();
    const promJson = parsePrometheusTextFormat(text);
    console.info("Fetched metrics..", promJson);
    return promJson;
  }
}

export default PrometheusImport;
