import {
  html,
  render,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "https://esm.sh/htm/preact/standalone";
import MetricBuffer from "./common/MetricBuffer.js";
import PrometheusImport from "./common/PrometheusImport.js";
import debounce from "./common/debounce.js";
import ChartGrid from "./components/ChartGrid.js";

// Constants for debounce timing
const REFRESH_DEBOUNCE_MS = 500;
const SEARCH_DEBOUNCE_MS = 300;
const BUFFER_SIZE_DEBOUNCE_MS = 300;
const METRIC_BUFFER_SIZE = 10;
const METRIC_ENDPOINT = "./metrics";
const MIN_REFRESH_RATE = 250;

// Create singleton instances outside of the component to avoid recreating on each render
const metricBuffer = new MetricBuffer(METRIC_BUFFER_SIZE);
const prometheusImporter = new PrometheusImport(METRIC_ENDPOINT);

function App(props) {
  const [refreshRate, setRefreshRate] = useState(1000);
  const [searchValue, setSearchValue] = useState("");
  const [bufferSize, setBufferSize] = useState(METRIC_BUFFER_SIZE);
  const [debouncedRefreshRate, setDebouncedRefreshRate] = useState(refreshRate);
  const [debouncedSearchValue, setDebouncedSearchValue] = useState(searchValue);
  const [debouncedBufferSize, setDebouncedBufferSize] = useState(bufferSize);

  // Use useMemo for expensive calculations that depend on specific inputs
  const debouncedSetRefreshRate = useMemo(
    () =>
      debounce((value) => {
        setDebouncedRefreshRate(value);
      }, REFRESH_DEBOUNCE_MS),
    [] // Empty dependency array means this is created only once
  );

  const debouncedSetSearchValue = useMemo(
    () =>
      debounce((value) => {
        setDebouncedSearchValue(value);
      }, SEARCH_DEBOUNCE_MS),
    [] // Empty dependency array means this is created only once
  );

  const debouncedSetBufferSize = useMemo(
    () =>
      debounce((value) => {
        setDebouncedBufferSize(value);
      }, BUFFER_SIZE_DEBOUNCE_MS),
    [] // Empty dependency array means this is created only once
  );

  // Use a more efficient event handler
  const handleRefreshRateChange = useCallback((e) => {
    const value = Math.max(MIN_REFRESH_RATE, Number.parseInt(e.target.value) || MIN_REFRESH_RATE);
    setRefreshRate(value);
  }, []);

  const handleSearchChange = useCallback((e) => {
    setSearchValue(e.target.value);
  }, []);

  const handleBufferSizeChange = useCallback((e) => {
    const value = Math.max(1, Number.parseInt(e.target.value) || 1);
    setBufferSize(value);
  }, []);

  useEffect(() => {
    debouncedSetRefreshRate(refreshRate);
  }, [refreshRate, debouncedSetRefreshRate]);

  useEffect(() => {
    debouncedSetSearchValue(searchValue);
  }, [searchValue, debouncedSetSearchValue]);

  useEffect(() => {
    debouncedSetBufferSize(bufferSize);
  }, [bufferSize, debouncedSetBufferSize]);

  return html`
    <div class="container">
      <h1>Metrics</h1>
      <section class="grid">
        <label>
          Filter by prefix
          <input
            type="search"
            name="filter-by-prefix"
            placeholder="Filter by prefix"
            aria-label="Text"
            value=${searchValue}
            onInput=${handleSearchChange}
          />
        </label>
        <label>
          Refresh rate (ms) [min = 250ms]
          <input
            type="number"
            name="refresh-rate"
            placeholder="Refresh rate (ms) [min = 250]"
            aria-label="Number"
            value=${refreshRate}
            onInput=${handleRefreshRateChange}
            min=${MIN_REFRESH_RATE}
          />
        </label>
        <label>
          Buffer size
          <input
            type="number"
            name="buffer-size"
            placeholder="Buffer size"
            aria-label="Number"
            value=${bufferSize}
            onInput=${handleBufferSizeChange}
            min="1"
          />
        </label>
      </section>
      <section>
        <${ChartGrid}
          searchValue=${debouncedSearchValue}
          refreshRate=${debouncedRefreshRate}
          bufferSize=${debouncedBufferSize}
        />
      </section>
    </div>
  `;
}

render(html`<${App} name="World" />`, document.body);
