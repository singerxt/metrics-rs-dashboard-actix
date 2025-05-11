import {
  html,
  render,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "https://esm.sh/htm/preact/standalone";
import apexDefaultTheme from "./common/apexDefaultTheme.js";
import debounce from "./common/debounce.js";
import ChartGrid from "./components/ChartGrid.js";

/**
 * Initializes the default Apex chart theme
 */
apexDefaultTheme();

// Constants for debounce timing
/**
 * Time in milliseconds to debounce refresh rate changes
 * @constant {number}
 */
const REFRESH_DEBOUNCE_MS = 500;

/**
 * Time in milliseconds to debounce search input changes
 * @constant {number}
 */
const SEARCH_DEBOUNCE_MS = 300;

/**
 * Time in milliseconds to debounce buffer size changes
 * @constant {number}
 */
const BUFFER_SIZE_DEBOUNCE_MS = 300;

/**
 * Default size of the metric buffer (history length)
 * @constant {number}
 */
const METRIC_BUFFER_SIZE_DEFAULT = 10;

/**
 * Minimum allowed refresh rate in milliseconds
 * @constant {number}
 */
const MIN_REFRESH_RATE = 100;

/**
 * Main application component
 *
 * @param {Object} props - Component properties
 * @returns {import("preact").VNode} Rendered component
 */
function App(props) {
  // State for user inputs
  /**
   * Current refresh rate in milliseconds
   * @type {[number, Function]}
   */
  const [refreshRate, setRefreshRate] = useState(1000);

  /**
   * Current search filter value
   * @type {[string, Function]}
   */
  const [searchValue, setSearchValue] = useState("");

  /**
   * Current buffer size (history length)
   * @type {[number, Function]}
   */
  const [bufferSize, setBufferSize] = useState(METRIC_BUFFER_SIZE_DEFAULT);

  // State for debounced values
  /**
   * Debounced refresh rate to reduce unnecessary updates
   * @type {[number, Function]}
   */
  const [debouncedRefreshRate, setDebouncedRefreshRate] = useState(refreshRate);

  /**
   * Debounced search value to reduce unnecessary filtering
   * @type {[string, Function]}
   */
  const [debouncedSearchValue, setDebouncedSearchValue] = useState(searchValue);

  /**
   * Debounced buffer size to reduce unnecessary resizing
   * @type {[number, Function]}
   */
  const [debouncedBufferSize, setDebouncedBufferSize] = useState(bufferSize);

  /**
   * Whether data collection is paused
   * @type {[boolean, Function]}
   */
  const [pause, setPause] = useState(false);

  /**
   * Debounced function to update refresh rate
   * @type {Function}
   */
  const debouncedSetRefreshRate = useMemo(
    () =>
      debounce((value) => {
        setDebouncedRefreshRate(value);
      }, REFRESH_DEBOUNCE_MS),
    [], // Empty dependency array means this is created only once
  );

  /**
   * Debounced function to update search value
   * @type {Function}
   */
  const debouncedSetSearchValue = useMemo(
    () =>
      debounce((value) => {
        setDebouncedSearchValue(value);
      }, SEARCH_DEBOUNCE_MS),
    [], // Empty dependency array means this is created only once
  );

  /**
   * Debounced function to update buffer size
   * @type {Function}
   */
  const debouncedSetBufferSize = useMemo(
    () =>
      debounce((value) => {
        setDebouncedBufferSize(value);
      }, BUFFER_SIZE_DEBOUNCE_MS),
    [], // Empty dependency array means this is created only once
  );

  /**
   * Handles refresh rate input changes
   * @param {Event} e - Input change event
   */
  const handleRefreshRateChange = useCallback((e) => {
    const value = Math.max(
      MIN_REFRESH_RATE,
      Number.parseInt(e.target.value) || MIN_REFRESH_RATE,
    );
    setRefreshRate(value);
  }, []);

  /**
   * Handles search input changes
   * @param {Event} e - Input change event
   */
  const handleSearchChange = useCallback((e) => {
    setSearchValue(e.target.value);
  }, []);

  /**
   * Handles buffer size input changes
   * @param {Event} e - Input change event
   */
  const handleBufferSizeChange = useCallback((e) => {
    const value = Math.max(1, Number.parseInt(e.target.value) || 1);
    setBufferSize(value);
  }, []);

  // Effect hooks to trigger debounced updates when values change
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
    <div class="">
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
          Refresh rate (ms) [min = 100ms]
          <input
            type="number"
            name="refresh-rate"
            placeholder="Refresh rate (ms) [min = 100]"
            aria-label="Number"
            value=${refreshRate}
            onInput=${handleRefreshRateChange}
            min=${MIN_REFRESH_RATE}
          />
        </label>
        <label>
          Buffer size (aka history length)
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
        <label>
          <button onClick=${() => setPause(!pause)} class="container">
            ${pause ? "play" : "pause"}
          </button>
        </label>
      </section>
      <section>
        <${ChartGrid}
          searchValue=${debouncedSearchValue}
          refreshRate=${debouncedRefreshRate}
          bufferSize=${debouncedBufferSize}
          pause=${pause}
        />
      </section>
    </div>
  `;
}

render(html`<${App} name="World" />`, document.body);
