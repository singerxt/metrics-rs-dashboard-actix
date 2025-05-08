class MetricBuffer {
  #buffer = new Map();
  // Maximum number of entries per metric
  #bufferSize = 10;

  constructor(bufferSize) {
    if (bufferSize) {
      this.#bufferSize = bufferSize;
    }
  }

  addMetrics(metrics) {
    for (const sample of metrics) {
      const currentSample = this.#buffer.get(sample.name);
      if (!currentSample) {
        this.#buffer.set(sample.name, sample);
        return;
      }

      if (currentSample.metrics.length >= this.#bufferSize) {
        currentSample.metrics.shift();
      }

      const newMetrics = currentSample.metrics.concat(sample.metrics);
      currentSample.metrics = newMetrics;
      this.#buffer.set(sample.name, currentSample);
    }
  }

  logMetrics() {
    for (const [name, sample] of this.#buffer.entries()) {
      console.log(`Metric: ${name}`);
      console.log(`  Type: ${sample.type}`);
      console.log(`  Help: ${sample.help}`);
      console.log("  Metrics:");
      for (const metric of sample.metrics) {
        console.log(`    ${JSON.stringify(metric)}`);
      }
    }
  }
}

export default MetricBuffer;
