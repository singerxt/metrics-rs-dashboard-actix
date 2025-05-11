/**
 * Class for buffering and managing metrics data.
 * Stores metrics in a Map with limited buffer size per metric.
 */
class MetricBuffer {
  /** @private Map to store metric samples */
  #buffer = new Map();
  /** @private Maximum number of entries per metric */
  #bufferSize = 10;

  /**
   * Creates a new MetricBuffer instance
   * @param {number} bufferSize - Optional custom buffer size for metrics
   */
  constructor(bufferSize) {
    if (bufferSize) {
      this.#bufferSize = bufferSize;
    }
  }

  /**
   * Calculates the total buffer size based on unique label types
   * @private
   * @param {Array} metrics - Array of metric data points
   * @returns {number} The calculated buffer size
   */
  calculateBufferSize(metrics) {
    const uniqueLabels = new Set();
    for (const sample of metrics) {
      if (sample?.labels?.type) {
        uniqueLabels.add(sample.labels.type);
      }
    }
    return this.#bufferSize * (uniqueLabels.size || 1);
  }

  /**
   * Adds new metrics to the buffer, maintaining the buffer size limit
   * @param {Array} metrics - Array of metric samples to add
   * @param {string} metrics[].name - Name of the metric
   * @param {string} metrics[].type - Type of the metric
   * @param {string} metrics[].help - Help text for the metric
   * @param {Array} metrics[].metrics - Array of metric data points
   */
  addMetrics(metrics) {
    for (const sample of metrics) {
      if (sample) {
        for (const metric of sample.metrics) {
          if (metric) {
            metric.timestamp = Date.now();
          }
        }
      }

      const currentSample = this.#buffer.get(sample.name);
      if (!currentSample) {
        this.#buffer.set(sample.name, sample);
        return;
      }

      if (
        currentSample.metrics.length >= this.calculateBufferSize(sample.metrics)
      ) {
        currentSample.metrics.shift();
      }

      const newMetrics = currentSample.metrics.concat(sample.metrics);
      currentSample.metrics = newMetrics;
      this.#buffer.set(sample.name, currentSample);
    }
  }

  /**
   * Sets a new buffer size for the metric buffer
   * @param {number} bufferSize - The new buffer size to set
   * @throws {Warning} - If buffer size is not greater than 10
   */
  setBufferSize(bufferSize) {
    if (bufferSize > 10) {
      this.#bufferSize = bufferSize;
    } else {
      console.warn("Buffer size must be greater than 10");
    }
  }

  /**
   * Logs all stored metrics to the console
   * Displays name, type, help text, and all stored metric data points
   */
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

  /**
   * Returns all metrics stored in the buffer
   * @returns {Array} Array of all metric samples
   */
  getMetrics() {
    return Array.from(this.#buffer.values());
  }
}

export default MetricBuffer;
