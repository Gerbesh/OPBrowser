var counter = 0;
document.getElementById("control").addEventListener("click", function(event) {
  counter = counter + 1;
  document.getElementById("status").textContent = "Clicks: " + counter;
});
