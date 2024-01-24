"use strict";

/**
 *
 * @param {Event} event
 */
function uncheck_counter(event) {
  /**
   * @param {HTMLElement}
   */
  const target = event.target;
  if (!target.checked) {
    return;
  }

  /**
   * @type(string | undefined)
   */
  const opp_element = target.dataset.opp;
  if (opp_element) {
    /**
     * @type(HtmlElement | undefined)
     */
    const oppelement = document.querySelector(`#${opp_element}`);

    if (oppelement) {
      oppelement.checked = false;
    }
  }
}

const excelFileField = document.querySelector("#excelFile");
const excelFileName = document.querySelector("#fileName");
const contractionFile = document.querySelector("#contraFile");
const uploadStat = document.querySelector("#uploadStat");
const startJob = document.querySelector("#startJob");

excelFileField.addEventListener("change", function (e) {
  const target = e.target;
  if (target.files.length == 0) {
    excelFileName.textContent = "No file selected";
    startJob.disabled = true;
  } else {
    excelFileName.textContent = target.files[0].name;
    startJob.disabled = false;
  }
});

contractionFile.addEventListener("change", function (e) {
  const target = e.target;
  if (target.files.length == 0) {
    uploadStat.classList.remove("uploaded");
  } else {
    uploadStat.classList.add("uploaded");
  }
});
