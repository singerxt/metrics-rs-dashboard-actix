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
    if (!metrics || !Array.isArray(metrics)) {
      return this.#bufferSize;
    }

    const uniqueLabels = new Set();
    try {
      for (const sample of metrics) {
        if (sample?.labels?.type) {
          uniqueLabels.add(sample.labels.type);
        } else if (sample?.labels?.phase) {
          uniqueLabels.add(sample.labels.phase);
        } else if (sample?.labels?.pattern) {
          uniqueLabels.add(sample.labels.pattern);
        }
      }
      return this.#bufferSize * (uniqueLabels.size || 1);
    } catch (error) {
      console.warn("Error calculating buffer size", error);
      return this.#bufferSize;
    }
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
    if (!metrics || !Array.isArray(metrics)) {
      console.warn("Invalid metrics data provided to addMetrics");
      return;
    }

    for (const sample of metrics) {
      if (
        !sample ||
        !sample.name ||
        !sample.metrics ||
        !Array.isArray(sample.metrics)
      ) {
        console.warn("Invalid sample in metrics data", sample);
        continue;
      }

      try {
        // Add timestamp to each metric point
        for (const metric of sample.metrics) {
          if (metric) {
            metric.timestamp = Date.now();
          }
        }

        const currentSample = this.#buffer.get(sample.name);
        if (!currentSample) {
          this.#buffer.set(sample.name, sample);
          continue; // Changed from return to continue to process all samples
        }

        if (
          currentSample.metrics.length >=
          this.calculateBufferSize(sample.metrics)
        ) {
          currentSample.metrics.shift();
        }

        const newMetrics = currentSample.metrics.concat(sample.metrics);
        currentSample.metrics = newMetrics;
        this.#buffer.set(sample.name, currentSample);
      } catch (error) {
        console.error("Error adding metric sample:", error, sample);
      }
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
    try {
      return Array.from(this.#buffer.values()).filter(
        (sample) =>
          sample &&
          sample.name &&
          sample.metrics &&
          Array.isArray(sample.metrics),
      );
    } catch (error) {
      console.error("Error retrieving metrics:", error);
      return [];
    }
  }
}

export default MetricBuffer;
